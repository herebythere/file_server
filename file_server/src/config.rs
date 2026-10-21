use std::env;
use std::path::PathBuf;

pub struct Config {
    pub host: String,
    pub port: String,
    pub directory: PathBuf,
    pub content_encodings: Vec<String>,
}

impl Config {
    pub fn new() -> Result<Config, std::io::Error> {
        let curr_dir = match env::current_dir() {
            Ok(pb) => pb,
            Err(e) => return Err(e),
        };

        Ok(Config {
            host: "127.0.0.1".to_string(),
            port: "3000".to_string(),
            directory: curr_dir,
            content_encodings: Vec::new(),
        })
    }

    pub async fn try_from(args: Vec<String>) -> Result<Config, std::io::Error> {
        let mut config = match Config::new() {
            Ok(conf) => conf,
            Err(e) => return Err(e),
        };

        get_params_from_args(&mut config, args);

        Ok(config)
    }
}

fn get_params_from_args(config: &mut Config, args: Vec<String>) {
    for arg in args {
        let trimmed_arg = arg.strip_prefix("--host=");
        if let Some(host) = trimmed_arg {
            config.host = host.to_string();
            continue;
        }

        let trimmed_arg = arg.strip_prefix("--port=");
        if let Some(port) = trimmed_arg {
            config.port = port.to_string();
            continue;
        }

        let trimmed_arg = arg.strip_prefix("--directory=");
        if let Some(directory) = trimmed_arg {
            config.directory = PathBuf::from(directory);
            continue;
        }

        let trimmed_arg = arg.strip_prefix("--content-encodings=");
        if let Some(content_encodings) = trimmed_arg {
            let mut split_encodings: Vec<String> = Vec::new();
            for encoding in content_encodings.split(",") {
                split_encodings.push(encoding.to_string());
            }
            config.content_encodings = split_encodings;
        }
    }
}
