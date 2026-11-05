use crate::available_encodings::{add_extension, get_encodings};
use crate::content_type::get_content_type;
use crate::last_resort_response;
use crate::utils_flyweight::{
    BoxedResponse, NOT_FOUND_404, ResponseParams, get_url_path_from_request,
};
use http_body_util::{BodyExt, Full};
use hyper::StatusCode;
use hyper::body::Incoming;
use hyper::header::{ACCEPT_RANGES, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::http::{Request, Response};
use std::fs::Metadata;
use std::path::Path;
use tokio::fs;

pub async fn build_response(
    req: Request<Incoming>,
    res_params: ResponseParams,
) -> Result<BoxedResponse, hyper::http::Error> {
    let filepath = match get_url_path_from_request(&req, &res_params.directory, true) {
        Some(fp) => fp,
        _ => return last_resort_response::build_response(StatusCode::NOT_FOUND, NOT_FOUND_404),
    };

    let content_type = get_content_type(&filepath);
    let encodings = get_encodings(&req, &res_params.available_encodings);

    if let Some((metadata, encoding_opt)) = get_metadata_and_encoding(&filepath, encodings).await {
        return compose_response(&metadata, encoding_opt, content_type).await;
    };

    last_resort_response::build_response(StatusCode::NOT_FOUND, NOT_FOUND_404)
}

async fn get_metadata_and_encoding(
    filepath: &Path,
    encodings: Vec<String>,
) -> Option<(Metadata, Option<String>)> {
    for encoding in encodings {
        let encoded_path = match add_extension(&filepath, &encoding) {
            Some(enc_pth) => enc_pth,
            _ => continue,
        };

        let metadata = match fs::metadata(encoded_path).await {
            Ok(m) => m,
            _ => continue,
        };

        if metadata.is_file() {
            return Some((metadata, Some(encoding)));
        }
    }

    let metadata = match fs::metadata(filepath).await {
        Ok(m) => m,
        _ => return None,
    };

    match metadata.is_file() {
        true => Some((metadata, None)),
        _ => None,
    }
}

async fn compose_response(
    metadata: &Metadata,
    content_encoding: Option<String>,
    content_type: &str,
) -> Result<BoxedResponse, hyper::http::Error> {
    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT_RANGES, "bytes")
        .header(CONTENT_LENGTH, metadata.len());

    if let Some(enc) = content_encoding {
        builder = builder.header(CONTENT_ENCODING, enc);
    }

    builder.body(
        Full::new(bytes::Bytes::new())
            .map_err(|e| match e {})
            .boxed(),
    )
}
