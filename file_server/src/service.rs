use hyper::Request;
use hyper::body::Incoming;
use hyper::service::Service;
use response::{BoxedResponse, ResponseParams, compose_response};
use std::future::Future;
use std::pin::Pin;

#[derive(Clone, Debug)]
pub struct Svc {
    response_params: ResponseParams,
}

impl Svc {
    pub fn from(response_params: ResponseParams) -> Svc {
        Svc { response_params }
    }
}

impl Service<Request<Incoming>> for Svc {
    type Response = BoxedResponse;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let params = self.response_params.clone();
        Box::pin(async move { compose_response(req, params).await })
    }
}
