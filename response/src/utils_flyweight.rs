use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper::http::{Request, Response};
use std::path::{Component, PathBuf};
use tokio::io;

use crate::available_encodings::AvailableEncodings;

pub type BoxedResponse = Response<BoxBody<Bytes, io::Error>>;

pub const NOT_FOUND_404: &str = "404 not found";
pub const METHOD_NOT_ALLOWED_405: &str = "405 method not allowed";
pub const RANGE_NOT_SATISFIABLE_416: &str = "416 range not satisfiable";

#[derive(Clone, Debug)]
pub struct ResponseParams {
    pub directory: PathBuf,
    pub available_encodings: AvailableEncodings,
}

impl ResponseParams {
    pub fn from(directory: &PathBuf, content_encodings: &Vec<String>) -> ResponseParams {
        let available_encodings = AvailableEncodings::from(content_encodings);

        ResponseParams {
            directory: directory.clone(),
            available_encodings,
        }
    }
}

// https://doc.rust-lang.org/std/path/struct.Path.html#method.normalize_lexically
// normalize lexically in nightly
pub fn get_url_path_from_request(req: &Request<Incoming>, directory: &PathBuf) -> Option<PathBuf> {
    let uri_path = req.uri().path();
    let mut normalized_url_path = match normalize_uri_path_lexically(&uri_path) {
        Some(url_path) => url_path,
        _ => return None,
    };

    if req.uri().path().ends_with("/") {
        normalized_url_path.push("index.html");
    }

    let joined_path = directory.join(normalized_url_path);
    match joined_path.starts_with(directory) {
        true => Some(joined_path),
        _ => None,
    }
}

pub fn get_path_from_request(req: &Request<Incoming>, directory: &PathBuf) -> Option<PathBuf> {
    let uri_path = req.uri().path();
    let mut normalized_url_path = match normalize_uri_path_lexically(&uri_path) {
        Some(url_path) => url_path,
        _ => return None,
    };

    if req.uri().path().ends_with("/") {
        normalized_url_path.push("");
    }

    let joined_path = directory.join(normalized_url_path);
    match joined_path.starts_with(directory) {
        true => Some(joined_path),
        _ => None,
    }
}

pub fn normalize_uri_path_lexically(path_buf_str: &str) -> Option<PathBuf> {
    let path_buf = PathBuf::from(path_buf_str);

    let mut parts: Vec<Component> = Vec::new();
    for component in path_buf.components() {
        match component {
            Component::ParentDir => {
                if let None = parts.pop() {
                    return None;
                };
            }
            Component::Normal(_) => parts.push(component),
            _ => {}
        }
    }

    let mut normalized_uri_path = PathBuf::new();
    for component in parts {
        if let Component::Normal(os_str) = component {
            normalized_uri_path.push(os_str);
        }
    }

    Some(normalized_uri_path)
}
