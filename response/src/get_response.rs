use crate::available_encodings::{add_extension, get_encodings};
use crate::content_type::get_content_type;
use crate::last_resort_response;
use crate::range_response;
use crate::serialize_details;
use crate::utils_flyweight::{
    BoxedResponse, NOT_FOUND_404, ResponseParams, get_url_path_from_request,
};
use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::StatusCode;
use hyper::body::{Frame, Incoming};
use hyper::header::{CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE};
use hyper::http::{Request, Response};
use std::path::PathBuf;
use tokio::fs;
use tokio_util::io::ReaderStream;

pub async fn build_response(
    req: Request<Incoming>,
    res_params: ResponseParams,
) -> Result<BoxedResponse, hyper::http::Error> {
    // check for a json details request
    // should be on file
    if let Some(res) = serialize_details::build_response(&req, &res_params).await {
        return res;
    };

    // check for range request
    if let Some(res) = range_response::build_response(&req, &res_params).await {
        return res;
    }

    let encodings = get_encodings(&req, &res_params.available_encodings);
    if let Some(res) = build_req_path_response(&req, &res_params.directory, &encodings).await {
        return res;
    };

    last_resort_response::build_response(StatusCode::NOT_FOUND, NOT_FOUND_404)
}

async fn build_req_path_response(
    req: &Request<Incoming>,
    directory: &PathBuf,
    encodings: &Vec<String>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let filepath = match get_url_path_from_request(req, directory) {
        Some(fp) => fp,
        _ => return None,
    };

    let content_type = get_content_type(&filepath);
    for enc in encodings {
        let encoded_path = match add_extension(&filepath, enc) {
            Some(enc_pth) => enc_pth,
            _ => continue,
        };

        if let Some(res) = compose_response(&encoded_path, content_type, Some(enc)).await {
            return Some(res);
        }
    }

    compose_response(&filepath, content_type, None).await
}

async fn compose_response(
    filepath: &PathBuf,
    content_type: &str,
    content_encoding: Option<&str>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let file = match fs::File::open(filepath).await {
        Ok(m) => m,
        _ => return None,
    };

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
