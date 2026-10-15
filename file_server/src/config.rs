use std::env;
use std::path;
use std::path::PathBuf;
use tokio::fs;

use crate::errors::Error;

pub struct Config {
    pub host: String,
    pub port: String,
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
            host: "127.0.0.1".to_string(),
            port: "3000".to_string(),
            directory: curr_dir,
            content_encodings: None,
        })
    }

    // iterate though command line args
    // then no json needed
    pub async fn try_from(args: Vec<String>) -> Result<Config, Error> {
        let mut config = match Config::new() {
            Ok(conf) => conf,
            Err(e) => return Err(e),
        };

        Ok(config)
    }
}

// function for get_directory

// function for get gonctent_encodings

// let config_path = match path::absolute(&filepath) {
//     Ok(pb) => pb,
//     Err(e) => return Err(Error::Io(e)),
// };

// let parent_dir = match config_path.parent() {
//     Some(p) => p,
//     _ => {
//         return Err(Error::Custom(
//             "parent directory of config not found".to_string(),
//         ));
//     }
// };

// let target_directory =
//     match fs::canonicalize(parent_dir.join(config.directory.clone())).await {
//         Ok(pb) => pb,
//         Err(e) => return Err(Error::Io(e)),
//     };

// config.directory = target_directory;