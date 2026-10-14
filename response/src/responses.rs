use hyper::body::Incoming;
use hyper::http::Request;
use hyper::Method;
use hyper::StatusCode;

use crate::get_response;
use crate::head_response;
use crate::last_resort_response;
use crate::utils_flyweight::{BoxedResponse, ResponseParams, METHOD_NOT_ALLOWED_405};

pub async fn compose_response(
    req: Request<Incoming>,
    params: ResponseParams,
) -> Result<BoxedResponse, hyper::http::Error> {
    match req.method() {
        &Method::GET => get_response::build_response(req, params).await,
        &Method::HEAD => head_response::build_response(req, params).await,
        _ => last_resort_response::build_response(
            StatusCode::METHOD_NOT_ALLOWED,
            METHOD_NOT_ALLOWED_405,
        ),
    }
}
