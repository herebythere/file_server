use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use response::ResponseParams;
use std::env;
use std::path::PathBuf;
use tokio::net::TcpListener;

mod config;
mod errors;
mod service;

use config::Config;
use errors::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let conf = match get_config().await {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    let host_and_port = conf.host.to_string() + ":" + &conf.port;
    let response_params = ResponseParams::from(&conf.directory, &conf.content_encodings);
    let svc = service::Svc::from(response_params);

    let listener = match TcpListener::bind(&host_and_port).await {
        Ok(lstnr) => lstnr,
        Err(e) => return Err(Error::Io(e)),
    };

    println!("file_server @ http://{}", host_and_port);

    loop {
        let (stream, _remote_address) = match listener.accept().await {
            Ok(strm) => strm,
            Err(e) => {
                println!("Server connection error:\n{}", e);
                continue;
            }
        };

        let io = TokioIo::new(stream);
        let svc = svc.clone();

        tokio::task::spawn(async move {
            if let Err(e) = Builder::new(TokioExecutor::new())
                .serve_connection(io, svc)
                .await
            {
                println!("{}", e);
            }
        });
    }
}

async fn get_config() -> Result<Config, Error> {
    match env::args().nth(1) {
        Some(conf_path_arg) => {
            let conf_pathbuf = PathBuf::from(conf_path_arg);
            return Config::try_from(&conf_pathbuf).await;
        }
        _ => Config::new(),
    }
}
