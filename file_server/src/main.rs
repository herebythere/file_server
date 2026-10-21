use config::Config;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use response::ResponseParams;
use std::env;
use tokio::net::TcpListener;

mod config;
mod service;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let args = env::args().collect();
    let conf = match Config::try_from(args).await {
        Ok(c) => c,
        Err(e) => return Err(e),
    };

    let host_and_port = conf.host.to_string() + ":" + &conf.port;
    let response_params = ResponseParams::from(&conf.directory, &conf.content_encodings);
    let svc = service::Svc::from(response_params);

    let listener = match TcpListener::bind(&host_and_port).await {
        Ok(lstnr) => lstnr,
        Err(e) => return Err(e),
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
