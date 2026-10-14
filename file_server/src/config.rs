use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::path;
use std::path::PathBuf;
use tokio::fs;

use crate::errors::Error;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub host_and_port: String,
    pub directory: PathBuf,
    pub content_encodings: Option<Vec<String>>,
}

impl Config {
    pub fn new() -> Result<Config, Error> {
        let curr_dir = match env::current_dir() {
            Ok(pb) => pb,
            Err(e) => return Err(Error::Io(e)),
        };

        Ok(Config {
            host_and_port: "127.0.0.1:3000".to_string(),
            directory: curr_dir,
            content_encodings: None,
        })
    }

    pub async fn try_from(filepath: &PathBuf) -> Result<Config, Error> {
        let config_json = match fs::read_to_string(filepath).await {
            Ok(r) => r,
            Err(e) => return Err(Error::Io(e)),
        };

        let mut config: Config = match serde_json::from_str(&config_json) {
            Ok(j) => j,
            Err(e) => return Err(Error::SerdeJson(e)),
        };

        // get target directory
        let config_path = match path::absolute(&filepath) {
            Ok(pb) => pb,
            Err(e) => return Err(Error::Io(e)),
        };

        let parent_dir = match config_path.parent() {
            Some(p) => p,
            _ => {
                return Err(Error::Custom(
                    "parent directory of config not found".to_string(),
                ));
            }
        };

        let target_directory =
            match fs::canonicalize(parent_dir.join(config.directory.clone())).await {
                Ok(pb) => pb,
                Err(e) => return Err(Error::Io(e)),
            };

        config.directory = target_directory;

        Ok(config)
    }
}
