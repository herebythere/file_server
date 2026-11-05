use crate::available_encodings::get_encodings;
use crate::content_type::get_content_type;
use crate::last_resort_response;
use crate::utils_flyweight::{
    BoxedResponse, NOT_FOUND_404, RANGE_NOT_SATISFIABLE_416, ResponseParams, get_file_and_encoding,
    get_url_path_from_request,
};
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::body::Incoming as IncomingBody;
use hyper::header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE};
use hyper::http::{Request, Response, StatusCode};
use std::io::SeekFrom;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;

pub async fn build_response(
    req: &Request<IncomingBody>,
    res_params: &ResponseParams,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let ranges = match get_ranges(req) {
        Some(rh) => rh,
        _ => return None,
    };

    match attempt_range_response(req, res_params, &ranges).await {
        Some(res) => Some(res),
        _ => Some(last_resort_response::build_response(
            StatusCode::NOT_FOUND,
            NOT_FOUND_404,
        )),
    }
}

async fn attempt_range_response(
    req: &Request<IncomingBody>,
    res_params: &ResponseParams,
    ranges: &Vec<(Option<usize>, Option<usize>)>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let filepath = match get_url_path_from_request(req, &res_params.directory, true) {
        Some(fp) => fp,
        _ => return None,
    };

    let content_type = get_content_type(&filepath);
    let encodings = get_encodings(&req, &res_params.available_encodings);
    let (file, encoding_opt) = match get_file_and_encoding(&filepath, &encodings).await {
        Some((f, e)) => (f, e),
        _ => return None,
    };

    compose_range_response(file, content_type, encoding_opt, &ranges).await
}

// on any fail return nothing
// split stuff up, save the "verify" on usage
fn get_ranges(req: &Request<IncomingBody>) -> Option<Vec<(Option<usize>, Option<usize>)>> {
    let accept_encoding_header = match req.headers().get(RANGE) {
        Some(enc) => enc,
        _ => return None,
    };

    let header_str = match accept_encoding_header.to_str() {
        Ok(hstr) => hstr,
        _ => return None,
    };

    let ranges_str = match header_str.trim().strip_prefix("bytes=") {
        Some(r) => r,
        _ => return None,
    };

    let mut ranges: Vec<(Option<usize>, Option<usize>)> = Vec::new();
    for range_value_str in ranges_str.split(",") {
        let range_str = range_value_str.trim();

        // prefix range
        if let Some(suffexless) = range_str.strip_suffix("-") {
            let start_range_int: usize = match suffexless.parse() {
                Ok(sri) => sri,
                _ => return None,
            };

            // add verified ranges here

            ranges.push((Some(start_range_int), None));
            continue;
        }

        // suffix-range
        if let Some(prefixless) = range_str.strip_prefix("-") {
            let end_range_int: usize = match prefixless.parse() {
                Ok(sri) => sri,
                _ => return None,
            };

            ranges.push((None, Some(end_range_int)));
            continue;
        }

        // window-range
        let start_end_range = match get_window_range(range_str) {
            Some(ser) => ser,
            _ => return None,
        };

        ranges.push(start_end_range)
    }

    if 0 < ranges.len() {
        return Some(ranges);
    }

    None
}

fn get_window_range(range_chunk: &str) -> Option<(Option<usize>, Option<usize>)> {
    let mut values = range_chunk.split("-");

    let start_range_str = match values.next() {
        Some(start_range) => start_range,
        _ => return None,
    };

    let end_range_str = match values.next() {
        Some(end_range) => end_range,
        _ => return None,
    };

    let start_range_int: usize = match start_range_str.parse() {
        Ok(sri) => sri,
        _ => return None,
    };

    let end_range_int: usize = match end_range_str.parse() {
        Ok(sri) => sri,
        _ => return None,
    };

    if start_range_int < end_range_int {
        return Some((Some(start_range_int), Some(end_range_int)));
    }

    None
}

async fn compose_range_response(
    file: File,
    content_type: &str,
    content_encoding: Option<String>,
    ranges: &Vec<(Option<usize>, Option<usize>)>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    match ranges.len() {
        0 => Some(last_resort_response::build_response(
            StatusCode::RANGE_NOT_SATISFIABLE,
            RANGE_NOT_SATISFIABLE_416,
        )),
        1 => compose_single_range_response(file, content_type, content_encoding, &ranges).await,
        // vs multi range (stretch goal)
        _ => Some(last_resort_response::build_response(
            StatusCode::RANGE_NOT_SATISFIABLE,
            RANGE_NOT_SATISFIABLE_416,
        )),
    }
}

async fn compose_single_range_response(
    mut file: File,
    content_type: &str,
    content_encoding: Option<String>,
    ranges: &Vec<(Option<usize>, Option<usize>)>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let metadata = match file.metadata().await {
        Ok(m) => m,
        _ => return None,
    };

    if !metadata.is_file() {
        return None;
    }

    // split here for single range response
    let size = metadata.len() as usize;
    let range = ranges.get(0);
    let (start, end) = match get_start_and_end(range, size) {
        Some(se) => se,
        _ => {
            return Some(last_resort_response::build_response(
                StatusCode::RANGE_NOT_SATISFIABLE,
                RANGE_NOT_SATISFIABLE_416,
            ));
        }
    };

    if let Err(_err) = file.seek(SeekFrom::Start(start.clone() as u64)).await {
        return None;
    };

    let content_range_header = build_content_range_header_str(start, end, size);
    let size = end - start;

    let mut builder = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(CONTENT_TYPE, content_type)
        .header(CONTENT_RANGE, content_range_header)
        .header(CONTENT_LENGTH, size);

    if let Some(enc) = content_encoding {
        builder = builder.header(CONTENT_ENCODING, enc);
    }

    let reader_stream = ReaderStream::with_capacity(file, size);
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    let boxed_body = stream_body.boxed();

    Some(builder.body(boxed_body))
}

fn get_start_and_end(
    range: Option<&(Option<usize>, Option<usize>)>,
    size: usize,
) -> Option<(usize, usize)> {
    let (start, end) = match range {
        // suffix (S - N, S)
        Some((None, Some(end))) => (size - end, size),
        // prefix (N, S)
        Some((Some(start), None)) => (start.clone(), size),
        // windowed (N, M)
        Some((Some(start), Some(end))) => (start.clone(), end.clone()),
        _ => return None,
    };

    if start <= end && end <= size {
        return Some((start, end));
    }

    None
}

fn build_content_range_header_str(start: usize, end: usize, size: usize) -> String {
    "bytes ".to_string() + &start.to_string() + "-" + &end.to_string() + "/" + &size.to_string()
}

// fn compose_multipart_range_request() {
//         // let mut buffer: Vec<u8> = Vec::with_capacity(end - start);
//     // buffer.resize(end - start, 0);
// }
