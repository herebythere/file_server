use crate::available_encodings::get_encodings;
use crate::content_type::get_content_type;
use crate::last_resort_response;
use crate::range_response;
use crate::response_params::ResponseParams;
use crate::serialize_details;
use crate::url_path_utils::{
    BoxedResponse, NOT_FOUND_404, get_file_and_encoding, get_url_path_from_request,
};
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::StatusCode;
use hyper::body::{Frame, Incoming};
use hyper::header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::http::{Request, Response};
use std::path::Path;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

pub async fn build_response(
    req: Request<Incoming>,
    res_params: ResponseParams,
) -> Result<BoxedResponse, hyper::http::Error> {
    // check for a json details request
    if let Some(res) = serialize_details::build_response(&req, &res_params).await {
        return res;
    };

    // check for range request
    if let Some(res) = range_response::build_response(&req, &res_params).await {
        return res;
    }

    // a humble get request
    let encodings = get_encodings(&req, &res_params.available_encodings);
    if let Some(res) = build_response_from_url_path(&req, &res_params.directory, &encodings).await {
        return res;
    };

    last_resort_response::build_response(StatusCode::NOT_FOUND, NOT_FOUND_404)
}

async fn build_response_from_url_path(
    req: &Request<Incoming>,
    directory: &Path,
    encodings: &Vec<String>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let filepath = match get_url_path_from_request(req, directory, true) {
        Some(fp) => fp,
        _ => return None,
    };

    let (file, encoding_opt) = match get_file_and_encoding(&filepath, encodings).await {
        Some((f, e)) => (f, e),
        _ => return None,
    };

    let content_type = get_content_type(&filepath);
    compose_response(file, content_type, encoding_opt).await
}

async fn compose_response(
    file: File,
    content_type: &str,
    content_encoding: Option<String>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let metadata = match file.metadata().await {
        Ok(m) => m,
        _ => return None,
    };

    if !metadata.is_file() {
        return None;
    }

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .header(CONTENT_LENGTH, metadata.len());

    if let Some(enc) = content_encoding {
        builder = builder.header(CONTENT_ENCODING, enc);
    }

    // https://github.com/hyperium/hyper/blob/master/examples/send_file.rs
    let reader_stream = ReaderStream::new(file);
    let stream_body = StreamBody::new(reader_stream.map_ok(Frame::data));
    let boxed_body = stream_body.boxed();

    Some(builder.body(boxed_body))
}
