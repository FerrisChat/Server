use axum::body::{self, BoxBody};
use axum::response::IntoResponse;
use http::header::CONTENT_TYPE;
use http::{HeaderValue, Response};
use serde::Serialize;

pub struct Json<T: Serialize> {
    pub obj: T,
    pub code: u16,
}

impl<T> Json<T>
where
    T: Serialize,
{
    #[inline]
    pub fn new(obj: T, code: u16) -> Self {
        Self { obj, code }
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response<BoxBody> {
        let bytes = match simd_json::to_vec(&self.obj) {
            Ok(res) => res,
            Err(err) => {
                return Response::builder()
                    .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                    .header(CONTENT_TYPE, HeaderValue::from_static("text/plain"))
                    .body(body::boxed(body::Full::from(err.to_string())))
                    .expect("failed to convert static data to a valid request");
            }
        };

        axum::http::Response::builder()
            .status(self.code)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(body::boxed(body::Full::from(bytes)))
            .unwrap_or_else(|e| {
                // this should only be reachable if a invalid HTTP code is passed in
                unreachable!(
                    "got an error while attempting to construct HTTP response for ServerError: {}",
                    e
                )
            })
    }
}

/*
fn json_content_type<B>(req: &RequestParts<B>) -> Result<bool, HeadersAlreadyExtracted> {
    let content_type = if let Some(content_type) = req
        .headers()
        .ok_or(HeadersAlreadyExtracted)?
        .get(header::CONTENT_TYPE)
    {
        content_type
    } else {
        return Ok(false);
    };

    let content_type = if let Ok(content_type) = content_type.to_str() {
        content_type
    } else {
        return Ok(false);
    };

    let mime = if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime
    } else {
        return Ok(false);
    };

    let is_json_content_type = mime.type_() == "application"
        && (mime.subtype() == "json" || mime.suffix().filter(|name| *name == "json").is_some());

    Ok(is_json_content_type)
}

#[async_trait]
impl<T, B> FromRequest<B> for Json<T>
where
    T: DeserializeOwned,
    B: http_body::Body + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = JsonRejection;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        use bytes::Buf;

        if json_content_type(req)? {
            let body = req.take_body().ok_or(BodyAlreadyExtracted)?;

            let value = simd_json::from_slice(buf.reader()).map_err(InvalidJsonBody::from_err)?;

            Ok(Json(value))
        } else {
            Err(MissingJsonContentType.into())
        }
    }
}
*/
