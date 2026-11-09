use crate::available_encodings::AvailableEncodings;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ResponseParams {
    pub directory: PathBuf,
    pub available_encodings: AvailableEncodings,
}

impl ResponseParams {
    pub fn from(directory: &Path, content_encodings: &Vec<String>) -> ResponseParams {
        let available_encodings = AvailableEncodings::from(content_encodings);

        ResponseParams {
            directory: directory.to_path_buf(),
            available_encodings,
        }
    }
}
