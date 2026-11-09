use crate::available_encodings::add_extension;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::body::Incoming;
use hyper::http::{Request, Response};
use std::path::{Component, MAIN_SEPARATOR, Path, PathBuf};
use tokio::fs::File;
use tokio::io;

pub type BoxedResponse = Response<BoxBody<Bytes, io::Error>>;

pub const NOT_FOUND_404: &str = "404 not found";
pub const METHOD_NOT_ALLOWED_405: &str = "405 method not allowed";
pub const RANGE_NOT_SATISFIABLE_416: &str = "416 range not satisfiable";

// https://doc.rust-lang.org/std/path/struct.Path.html#method.normalize_lexically
// normalize lexically in nightly
pub fn get_url_path_from_request(
    req: &Request<Incoming>,
    directory: &Path,
    add_index_on_directory: bool,
) -> Option<PathBuf> {
    let uri_path = req.uri().path();
    let url_path_as_os_path = convert_url_path_to_os_path(&uri_path, add_index_on_directory);
    let joined_path = directory.join(url_path_as_os_path);
    let normalized = normalize_path_lexicographically(&joined_path);

    match normalized.starts_with(directory) {
        true => Some(normalized),
        _ => None,
    }
}

fn convert_url_path_to_os_path(path_buf_str: &str, add_index_on_directory: bool) -> PathBuf {
    let mut normalized_uri_path = PathBuf::new();
    for split in path_buf_str.split("/") {
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

fn normalize_path_lexicographically(path_buf: &Path) -> PathBuf {
    let mut normalized_stack: Vec<Component> = Vec::new();
    for part in path_buf.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized_stack.pop();
            }
            _ => normalized_stack.push(part),
        }
    }

    let mut normalized = PathBuf::new();
    for part in normalized_stack {
        normalized.push(part);
    }

    normalized
}

pub async fn get_file_and_encoding(
    filepath: &Path,
    encodings: &Vec<String>,
) -> Option<(File, Option<String>)> {
    for encoding in encodings {
        let encoded_path = match add_extension(&filepath, &encoding) {
            Some(enc_pth) => enc_pth,
            _ => continue,
        };

        match File::open(encoded_path).await {
            Ok(f) => return Some((f, Some(encoding.clone()))),
            _ => continue,
        };
    }

    match File::open(filepath).await {
        Ok(f) => return Some((f, None)),
        _ => return None,
    }
}
