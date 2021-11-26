use axum::body::Full;
use axum::response::IntoResponse;
use bytes::Bytes;
use http::{header, HeaderValue, Response, StatusCode};
use serde::Serialize;
use std::convert::Infallible;

pub struct Json<T: Serialize> {
    obj: T,
    code: u16,
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    type Body = Full<Bytes>;
    type BodyError = Infallible;

    fn into_response(self) -> Response<Self::Body> {
        let bytes = match simd_json::to_vec(&self.obj) {
            Ok(res) => res,
            Err(err) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Full::from(err.to_string()))
                    .expect("failed to convert static data to a valid request");
            }
        };

        let mut res = Response::new(Full::from(bytes));
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        res
    }
}
