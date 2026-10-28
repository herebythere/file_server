use crate::available_encodings::AvailableEncodings;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper::http::{Request, Response};
use std::path::{Component, MAIN_SEPARATOR, Path, PathBuf};
use tokio::io;

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
pub fn get_url_path_from_request(
    req: &Request<Incoming>,
    directory: &PathBuf,
    add_index_on_directory: bool,
) -> Option<PathBuf> {
    let uri_path = req.uri().path();
    let url_path_as_os_path = convert_to_os_path(&uri_path, add_index_on_directory);

    let joined_path = directory.join(url_path_as_os_path);
    let normalized = match normalize_path_lexicographically(&joined_path) {
        Some(pth) => pth,
        _ => return None,
    };

    match normalized.starts_with(directory) {
        true => Some(normalized),
        _ => None,
    }
}

fn convert_to_os_path(path_buf_str: &str, add_index_on_directory: bool) -> PathBuf {
    let mut normalized_uri_path = PathBuf::new();

    let splitted = path_buf_str.split("/");
    for split in splitted {
        match split {
            "" => {}
            "." => normalized_uri_path.push(Component::CurDir),
            ".." => normalized_uri_path.push(Component::ParentDir),
            _ => normalized_uri_path.push(split),
        }
    }

    if add_index_on_directory && path_buf_str.ends_with(MAIN_SEPARATOR) {
        normalized_uri_path.push("index.html");
    }

    normalized_uri_path
}

// get url path from path(directory, pathbuf)
pub fn get_url_path_from_filepath(directory: &PathBuf, path_buf: &Path) -> Option<String> {
    let suffix = match path_buf.strip_prefix(directory) {
        Ok(sfx) => sfx,
        _ => return None,
    };

    let mut url_path: String = "".to_string();
    for component in suffix.components() {
        match component {
            Component::Normal(os) => {
                url_path.push_str("/");
                let os_str = match os.to_str() {
                    Some(st) => st,
                    _ => return None,
                };
                url_path.push_str(os_str);
            }
            _ => {}
        }
    }

    if path_buf.ends_with("/") {
        url_path.push_str("/");
    }

    Some(url_path)
}

// return with a path or an error
fn normalize_path_lexicographically(path_buf: &PathBuf) -> Option<PathBuf> {
    let mut normalized_stack: Vec<Component> = Vec::new();
    for part in path_buf.components() {
        match part {
            Component::ParentDir => {
                if let None = normalized_stack.pop() {
                    return None;
                }
            }
            Component::CurDir => {}
            _ => normalized_stack.push(part),
        }
    }

    let mut normalized = PathBuf::new();
    for part in normalized_stack {
        normalized.push(part);
    }

    Some(normalized)
}
