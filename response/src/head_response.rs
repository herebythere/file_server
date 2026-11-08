use crate::available_encodings::{add_extension, get_encodings};
use crate::content_type::get_content_type;
use crate::last_resort_response;
use crate::response_params::ResponseParams;
use crate::url_path_utils::{BoxedResponse, NOT_FOUND_404, get_url_path_from_request};
use http_body_util::{BodyExt, Full};
use hyper::StatusCode;
use hyper::body::Incoming;
use hyper::header::{ACCEPT_RANGES, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::http::{Request, Response};
use std::path::Path;
use tokio::fs;

pub async fn build_response(
    req: Request<Incoming>,
    res_params: ResponseParams,
) -> Result<BoxedResponse, hyper::http::Error> {
    if let Some(filepath) = get_url_path_from_request(&req, &res_params.directory, true) {
        let encodings = get_encodings(&req, &res_params.available_encodings);
        if let Some((size, encoding_opt)) = get_size_and_encoding(&filepath, encodings).await {
            let content_type = get_content_type(&filepath);
            return compose_response(content_type, size, encoding_opt).await;
        };
    };

    last_resort_response::build_response(StatusCode::NOT_FOUND, NOT_FOUND_404)
}

// just use "file" and grab metadata instead of more code?

async fn get_size_and_encoding(
    filepath: &Path,
    encodings: Vec<String>,
) -> Option<(u64, Option<String>)> {
    for encoding in encodings {
        let encoded_path = match add_extension(&filepath, &encoding) {
            Some(enc_pth) => enc_pth,
            _ => continue,
        };

        if let Some(size) = get_file_size(&encoded_path).await {
            return Some((size, Some(encoding.to_string())));
        }
    }

    match get_file_size(filepath).await {
        Some(size) => Some((size, None)),
        _ => None,
    }
}

async fn get_file_size(filepath: &Path) -> Option<u64> {
    let metadata = match fs::metadata(filepath).await {
        Ok(m) => m,
        _ => return None,
    };

    match metadata.is_file() {
        true => Some(metadata.len()),
        _ => None,
    }
}

async fn compose_response(
    content_type: &str,
    size: u64,
    content_encoding: Option<String>,
) -> Result<BoxedResponse, hyper::http::Error> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT_RANGES, "bytes")
        .header(CONTENT_LENGTH, size);

    if let Some(enc) = content_encoding {
        builder = builder.header(CONTENT_ENCODING, enc);
    }

    builder.body(
        Full::new(bytes::Bytes::new())
            .map_err(|e| match e {})
            .boxed(),
    )
}
