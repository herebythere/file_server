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
use std::path::Path;
use tokio::fs;

pub async fn build_response(
    req: Request<Incoming>,
    res_params: ResponseParams,
) -> Result<BoxedResponse, hyper::http::Error> {
    if let Some(filepath) = get_url_path_from_request(&req, &res_params.directory, true) {
        let content_type = get_content_type(&filepath);
        let encodings = get_encodings(&req, &res_params.available_encodings);

        // try encoded
        for encoding in encodings {
            let encoded_path = match add_extension(&filepath, &encoding) {
                Some(enc_pth) => enc_pth,
                _ => continue,
            };

            if let Some(res) = compose_response(&encoded_path, content_type, Some(&encoding)).await
            {
                return res;
            }
        }

        // try base
        if let Some(res) = compose_response(&filepath, content_type, None).await {
            return res;
        }
    };

    last_resort_response::build_response(StatusCode::NOT_FOUND, NOT_FOUND_404)
}

async fn compose_response(
    filepath: &Path,
    content_type: &str,
    content_encoding: Option<&str>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let metadata = match fs::metadata(filepath).await {
        Ok(m) => m,
        _ => return None,
    };

    if !metadata.is_file() {
        return None;
    }

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(ACCEPT_RANGES, "bytes")
        .header(CONTENT_LENGTH, metadata.len());

    if let Some(enc) = content_encoding {
        builder = builder.header(CONTENT_ENCODING, enc);
    }

    Some(
        builder.body(
            Full::new(bytes::Bytes::new())
                .map_err(|e| match e {})
                .boxed(),
        ),
    )
}
