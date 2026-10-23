use crate::content_type::HTML;
use crate::utils_flyweight::BoxedResponse;
use bytes;
use http_body_util::{BodyExt, Full};
use hyper::StatusCode;
use hyper::header::{CONTENT_TYPE, HeaderValue};
use hyper::http::Response;

pub fn build_response(
    status_code: StatusCode,
    body: &'static str,
) -> Result<BoxedResponse, hyper::http::Error> {
    Response::builder()
        .status(status_code)
        .header(CONTENT_TYPE, HeaderValue::from_static(HTML))
        .body(
            Full::new(bytes::Bytes::from(body))
                .map_err(|e| match e {})
                .boxed(),
        )
}
