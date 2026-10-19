use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{HeaderValue, CONTENT_TYPE};
use hyper::http::{Request, Response};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs::Metadata;
use std::io::Error as IoError;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs;

use crate::last_resort_response;
use crate::utils_flyweight::{get_path_from_request, BoxedResponse, ResponseParams, NOT_FOUND_404};

// Need to jouge up for some clean json stuff, option should be property exists or no not NULL
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FileDetails {
    is_dir: bool,
    file_name: String,
    url_path: PathBuf,
    size: u64,
    created_at: Option<u128>,
    accessed_at: Option<u128>,
    modified_at: Option<u128>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EntryDetails {
    details: FileDetails,
    entries: Vec<FileDetails>,
}

pub async fn build_response(
    req: &Request<Incoming>,
    res_params: &ResponseParams,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let params = match req.uri().query() {
        Some(prms) => prms,
        _ => return None,
    };

    if 0 == params.len() {
        return None;
    }

    let params_map = form_urlencoded::parse(params.as_bytes())
        .into_owned()
        .collect::<HashMap<String, String>>();

    let serialize_as = match params_map.get("details_as") {
        Some(srlz) => srlz,
        _ => return None,
    };

    if "json" != serialize_as {
        return None;
    }

    compose_entry_details_response(req, res_params).await
}

async fn compose_entry_details_response(
    req: &Request<Incoming>,
    res_params: &ResponseParams,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let req_path = match get_path_from_request(req, &res_params.directory) {
        Some(pth) => pth,
        _ => return None,
    };

    let details = get_entry_details(&res_params.directory, &req_path).await;

    match compose_response(details).await {
        Some(res) => Some(res),
        _ => Some(last_resort_response::build_response(
            StatusCode::NOT_FOUND,
            NOT_FOUND_404,
        )),
    }
}

pub async fn get_entry_details(directory: &PathBuf, uri_path: &PathBuf) -> Option<EntryDetails> {
    let metadata = match fs::metadata(&uri_path).await {
        Ok(m) => m,
        _ => return None,
    };

    // immediately drop symlink, feels dangerous
    if metadata.is_symlink() {
        return None;
    }

    match metadata.is_dir() {
        true => build_directory_entry(&metadata, &uri_path, directory).await,
        _ => build_file_entry(&metadata, &uri_path, directory),
    }
}

fn build_file_entry(
    metadata: &Metadata,
    req_path: &PathBuf,
    base_path: &PathBuf,
) -> Option<EntryDetails> {
    let details = create_entry_details(metadata, req_path, base_path);

    Some(EntryDetails {
        details,
        entries: Vec::new(),
    })
}

async fn build_directory_entry(
    metadata: &Metadata,
    req_path: &PathBuf,
    base_path: &PathBuf,
) -> Option<EntryDetails> {
    let details = create_entry_details(metadata, req_path, base_path);

    let mut entry_details = EntryDetails {
        details,
        entries: Vec::new(),
    };

    let mut entries = match fs::read_dir(req_path).await {
        Ok(entrs) => entrs,
        _ => return None,
    };

    while let Ok(opt_entry) = entries.next_entry().await {
        if let Some(entry) = opt_entry {
            let metadata = match entry.metadata().await {
                Ok(md) => md,
                _ => continue,
            };

            entry_details
                .entries
                .push(create_entry_details(&metadata, &entry.path(), base_path));

            continue;
        }

        break;
    }

    Some(entry_details)
}

fn create_entry_details(metadata: &Metadata, req_path: &Path, base_path: &PathBuf) -> FileDetails {
    let file_name = match req_path.file_name() {
        Some(flnm) => flnm.to_string_lossy().to_string(),
        None => "".to_string(),
    };

    let created_at = get_duration_since_as_ms(metadata.created());
    let accessed_at = get_duration_since_as_ms(metadata.accessed());
    let modified_at = get_duration_since_as_ms(metadata.modified());

    let url_path: PathBuf = match req_path.strip_prefix(base_path) {
        Ok(fp) => fp.to_path_buf(),
        Err(_) => PathBuf::from(""),
    };

    FileDetails {
        is_dir: metadata.is_dir(),
        file_name,
        url_path,
        created_at,
        accessed_at,
        modified_at,
        size: metadata.len(),
    }
}

async fn compose_response(
    details_opt: Option<EntryDetails>,
) -> Option<Result<BoxedResponse, hyper::http::Error>> {
    let details = match details_opt {
        Some(deets) => deets,
        _ => return None,
    };

    let body = match serde_json::to_string(&details) {
        Ok(bdy) => bdy,
        _ => return None,
    };

    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(
                Full::new(bytes::Bytes::from(body))
                    .map_err(|e| match e {})
                    .boxed(),
            ),
    )
}

fn get_duration_since_as_ms(timestamp_result: Result<SystemTime, IoError>) -> Option<u128> {
    if let Ok(timestamp) = timestamp_result {
        if let Ok(duration) = timestamp.duration_since(SystemTime::UNIX_EPOCH) {
            return Some(duration.as_millis());
        }
    }

    None
}
