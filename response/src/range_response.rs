use crate::available_encodings::get_encodings;
use crate::content_type::get_content_type;
use crate::last_resort_response;
use crate::response_params::ResponseParams;
use crate::url_path_utils::{
    BoxedResponse, NOT_FOUND_404, RANGE_NOT_SATISFIABLE_416, get_file_and_encoding,
    get_url_path_from_request,
};
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::body::Incoming as IncomingBody;
use hyper::header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE};
use hyper::http::{HeaderValue, Request, Response, StatusCode};
use std::io::SeekFrom;
use std::num::ParseIntError;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;

const MAX_RANGE_SIZE: u64 = 26214400;

pub async fn build_response(
    req: &Request<IncomingBody>,
    res_params: &ResponseParams,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let range_header = match req.headers().get(RANGE) {
        Some(enc) => enc,
        _ => return None,
    };

    match attempt_range_response(req, res_params, range_header).await {
        Some(res) => Some(res),
        _ => Some(last_resort_response::build_response(
            StatusCode::NOT_FOUND,
            NOT_FOUND_404,
        )),
    }
}

// Returning None fallsback to 404
async fn attempt_range_response(
    req: &Request<IncomingBody>,
    res_params: &ResponseParams,
    range_header: &HeaderValue,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    // THIS PART IS DOES THIS FILE EXIST
    let filepath = match get_url_path_from_request(req, &res_params.directory, true) {
        Some(fp) => fp,
        _ => return None,
    };

    let encodings = get_encodings(&req, &res_params.available_encodings);
    let (file, encoding_opt) = match get_file_and_encoding(&filepath, &encodings).await {
        Some((f, e)) => (f, e),
        _ => return None,
    };

    let metadata = match file.metadata().await {
        Ok(m) => m,
        _ => return None,
    };

    if !metadata.is_file() {
        return None;
    }

    let ranges = match get_ranges(range_header, metadata.len() - 1) {
        Some(rngs) => rngs,
        _ => {
            return Some(last_resort_response::build_response(
                StatusCode::RANGE_NOT_SATISFIABLE,
                RANGE_NOT_SATISFIABLE_416,
            ));
        }
    };

    let content_type = get_content_type(&filepath);

    match ranges.len() {
        0 => Some(last_resort_response::build_response(
            StatusCode::RANGE_NOT_SATISFIABLE,
            RANGE_NOT_SATISFIABLE_416,
        )),
        1 => {
            compose_single_range_response(file, metadata.len(), content_type, encoding_opt, ranges)
                .await
        }
        // vs multi range (stretch goal)
        _ => Some(last_resort_response::build_response(
            StatusCode::RANGE_NOT_SATISFIABLE,
            RANGE_NOT_SATISFIABLE_416,
        )),
    }
}

fn get_ranges(range_header: &HeaderValue, size: u64) -> Option<Vec<(u64, u64)>> {
    let header_str = match range_header.to_str() {
        Ok(hstr) => hstr,
        _ => return None,
    };

    let ranges_str = match header_str.trim().strip_prefix("bytes=") {
        Some(r) => r,
        _ => return None,
    };

    let mut ranges: Vec<(u64, u64)> = Vec::new();

    let mut split_ranges = ranges_str.split(",");

    // get first ranges to establish initial body size and tail / end of previous segment
    let (mut request_body_size, mut tail) = match split_ranges.next() {
        Some(range) => match build_window_range(range, size) {
            Some((start, end)) => {
                ranges.push((start, end));
                (end - start, end)
            }
            _ => return None,
        },
        _ => return None,
    };

    if MAX_RANGE_SIZE < request_body_size {
        return None;
    }

    // get other ranges
    while let Some(range) = split_ranges.next() {
        match build_window_range(range, size) {
            Some((start, end)) => {
                request_body_size += end - start;
                if MAX_RANGE_SIZE < request_body_size {
                    return None;
                }
                if start <= tail {
                    return None;
                }
                tail = end;

                ranges.push((start, end));
            }
            _ => return None,
        }
    }

    Some(ranges)
}

fn build_window_range(range_str: &str, size: u64) -> Option<(u64, u64)> {
    let values: Vec<&str> = range_str.split("-").collect();

    if 2 != values.len() {
        return None;
    }

    let first = values.get(0);
    let second = values.get(1);

    let (first_str, second_str) = match (first, second) {
        (Some(f), Some(s)) => (f, s),
        _ => return None,
    };

    let start_range_int: Result<u64, ParseIntError> = first_str.parse();
    let end_range_int: Result<u64, ParseIntError> = second_str.parse();

    match (start_range_int, end_range_int) {
        (Ok(start_int), Ok(mut end_int)) => {
            if size < end_int {
                end_int = size;
            }

            if start_int < end_int {
                return Some((start_int, end_int));
            }

            None
        }
        (Err(_), Ok(mut end_int)) => {
            if size < end_int {
                end_int = size;
            }
            Some((0, end_int))
        }
        (Ok(mut start_int), Err(_)) => {
            if size < start_int {
                start_int = size;
            }
            Some((start_int, size))
        }
        _ => None,
    }
}

async fn compose_single_range_response(
    mut file: File,
    file_size: u64,
    content_type: &str,
    content_encoding: Option<String>,
    ranges: Vec<(u64, u64)>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    // split here for single range response
    let (start, end) = match ranges.get(0) {
        Some((s, e)) => (s.clone(), e.clone()),
        _ => {
            return Some(last_resort_response::build_response(
                StatusCode::RANGE_NOT_SATISFIABLE,
                RANGE_NOT_SATISFIABLE_416,
            ));
        }
    };

    if let Err(_err) = file.seek(SeekFrom::Start(start.clone())).await {
        return None;
    };

    let content_range_header = build_content_range_header_str(start, end, file_size);
    let buffer_size = end - start + 1;

    let mut builder = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(CONTENT_TYPE, content_type)
        .header(CONTENT_RANGE, content_range_header)
        .header(CONTENT_LENGTH, buffer_size);

    if let Some(enc) = content_encoding {
        builder = builder.header(CONTENT_ENCODING, enc);
    }

    let reader_stream = ReaderStream::with_capacity(file, buffer_size as usize);
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    let boxed_body = stream_body.boxed();

    Some(builder.body(boxed_body))
}

fn build_content_range_header_str(start: u64, end: u64, file_size: u64) -> String {
    "bytes ".to_string() + &start.to_string() + "-" + &end.to_string() + "/" + &file_size.to_string()
}
