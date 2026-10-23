use crate::available_encodings::{add_extension, get_encodings};
use crate::content_type::get_content_type;
use crate::last_resort_response;
use crate::utils_flyweight::{
    BoxedResponse, NOT_FOUND_404, RANGE_NOT_SATISFIABLE_416, ResponseParams,
    get_url_path_from_request,
};
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::body::Incoming as IncomingBody;
use hyper::header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE};
use hyper::http::{Request, Response, StatusCode};
use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use tokio_util::io::ReaderStream;

pub async fn build_response(
    req: &Request<IncomingBody>,
    res_params: &ResponseParams,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let range_header = match get_range_header(req) {
        Some(rh) => rh,
        _ => return None,
    };

    // flatten this
    if let Some(filepath) = get_url_path_from_request(req, &res_params.directory, true) {
        if let Some(ranges) = get_ranges(&range_header) {
            let encodings = get_encodings(req, &res_params.available_encodings);

            if let Some(response) = build_single_range_response(&filepath, &encodings, ranges).await
            {
                return Some(response);
            }
        };
    }

    Some(last_resort_response::build_response(
        StatusCode::NOT_FOUND,
        NOT_FOUND_404,
    ))
}

fn get_range_header(req: &Request<IncomingBody>) -> Option<String> {
    let accept_encoding_header = match req.headers().get(RANGE) {
        Some(enc) => enc,
        _ => return None,
    };

    match accept_encoding_header.to_str() {
        Ok(s) => Some(s.to_string()),
        _ => None,
    }
}

// on any fail return nothing
fn get_ranges(range_header_value: &str) -> Option<Vec<(Option<usize>, Option<usize>)>> {
    let ranges_str = match range_header_value.trim().strip_prefix("bytes=") {
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

async fn build_single_range_response(
    filepath: &PathBuf,
    encodings: &Vec<String>,
    ranges: Vec<(Option<usize>, Option<usize>)>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let content_type = get_content_type(&filepath);

    if let Some(res) =
        compose_encoded_single_range_response(&filepath, content_type, &encodings, &ranges).await
    {
        return Some(res);
    };

    // origin target
    compose_single_range_response(&filepath, content_type, None, &ranges).await
}

async fn compose_encoded_single_range_response(
    filepath: &PathBuf,
    content_type: &str,
    encodings: &Vec<String>,
    ranges: &Vec<(Option<usize>, Option<usize>)>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    for enc in encodings {
        let encoded_path = match add_extension(filepath, &enc) {
            Some(enc_pth) => enc_pth,
            _ => continue,
        };

        if let Some(res) =
            compose_single_range_response(&encoded_path, content_type, Some(enc), ranges).await
        {
            return Some(res);
        }
    }

    None
}

async fn compose_single_range_response(
    filepath: &PathBuf,
    content_type: &str,
    content_encoding: Option<&str>,
    ranges: &Vec<(Option<usize>, Option<usize>)>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let mut file = match File::open(filepath).await {
        Ok(m) => m,
        _ => return None,
    };

    let metadata = match fs::metadata(filepath).await {
        Ok(m) => m,
        _ => return None,
    };

    if !metadata.is_file() {
        return None;
    }

    let size = metadata.len() as usize;
    let (start, end) = match get_start_and_end(ranges, size) {
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

    let mut buffer: Vec<u8> = Vec::with_capacity(end - start);
    buffer.resize(end - start, 0);

    let content_range_header = build_content_range_header_str(start, end, size);

    let mut builder = Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(CONTENT_TYPE, content_type)
        .header(CONTENT_RANGE, content_range_header)
        .header(CONTENT_LENGTH, buffer.len().to_string());

    if let Some(enc) = content_encoding {
        builder = builder.header(CONTENT_ENCODING, enc);
    }

    let reader_stream = ReaderStream::with_capacity(file, size);
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    let boxed_body = stream_body.boxed();

    Some(builder.body(boxed_body))
}

fn get_start_and_end(
    ranges: &Vec<(Option<usize>, Option<usize>)>,
    size: usize,
) -> Option<(usize, usize)> {
    let (start, end) = match ranges.get(0) {
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
