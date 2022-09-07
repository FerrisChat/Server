use axum::{
    http::{
        header::{HeaderMap, HeaderName, ACCEPT, CONTENT_TYPE},
        StatusCode,
    },
    response::{IntoResponse, Response as AxumResponse},
};
use common::CastSnowflakes;
use serde::Serialize;

/// An error response.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Error {
    /// Internal server error occured, this is likely a bug.
    InternalError {
        /// What caused the error. `None` if unknown.
        what: Option<String>,
        /// The error message.
        message: String,
        /// A debug version of the error, or `None` if there is no debug version.
        debug: Option<String>,
    },
}

/// A response to an endpoint.
#[derive(Debug)]
pub struct Response<T: Serialize>(
    /// The status code of the response.
    pub StatusCode,
    /// The body of the response. Should be serializable.
    pub T,
);

impl<T: Serialize> Response<T> {
    /// Creates a new successful response (200 OK) with the given body.
    pub const fn ok(body: T) -> Self {
        Self(StatusCode::OK, body)
    }

    /// Creates a new created response (201 Created) with the given body.
    pub const fn created(body: T) -> Self {
        Self(StatusCode::CREATED, body)
    }
}

impl<T: Serialize + CastSnowflakes> Response<T>
where
    T::StringResult: Serialize,
{
    /// Promotes this response to one that is aware of the given headers.
    pub fn promote(self, headers: &HeaderMap) -> HeaderAwareResponse<T> {
        HeaderAwareResponse(self, headers.clone())
    }
}

impl<T> Clone for Response<T>
where
    T: Serialize + Clone,
{
    fn clone(&self) -> Self {
        Self(self.0, self.1.clone())
    }
}

impl<T: Serialize> From<(u16, T)> for Response<T> {
    fn from((status_code, body): (u16, T)) -> Self {
        Self(
            StatusCode::from_u16(status_code).expect("invalid status code"),
            body,
        )
    }
}

impl<T: Serialize> From<(StatusCode, T)> for Response<T> {
    fn from((status_code, body): (StatusCode, T)) -> Self {
        Self(status_code, body)
    }
}

impl From<&str> for Response<Error> {
    fn from(message: &str) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Error::InternalError {
                what: None,
                message: message.into(),
                debug: None,
            },
        )
    }
}

impl From<String> for Response<Error> {
    fn from(message: String) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Error::InternalError {
                what: None,
                message,
                debug: None,
            },
        )
    }
}

impl From<sqlx::Error> for Response<Error> {
    fn from(err: sqlx::Error) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Error::InternalError {
                what: Some("database".into()),
                message: err.to_string(),
                debug: Some(format!("{:?}", err)),
            },
        )
    }
}

fn serialization_error(err: &(impl ToString + std::fmt::Debug)) -> AxumResponse {
    Response(
        StatusCode::INTERNAL_SERVER_ERROR,
        Error::InternalError {
            what: Some("serialization".into()),
            message: err.to_string(),
            debug: Some(format!("{:?}", err)),
        },
    )
    .into_response()
}

impl<T: Serialize> IntoResponse for Response<T> {
    fn into_response(self) -> AxumResponse {
        let bytes = match simd_json::to_vec(&self.1) {
            Ok(bytes) => bytes,
            // TODO: this could become infitite recursion
            Err(err) => {
                return serialization_error(&err);
            }
        };

        axum::http::Response::builder()
            .status(self.0)
            .header(CONTENT_TYPE, "application/json")
            .body(axum::body::Full::from(bytes))
            .expect("invalid http status code received")
            .into_response()
    }
}

/// A response that is aware of the request headers.
pub struct HeaderAwareResponse<T: Serialize + CastSnowflakes>(
    /// The response.
    pub Response<T>,
    /// The request headers.
    pub HeaderMap,
)
where
    T::StringResult: Serialize;

impl<T: Serialize + CastSnowflakes> HeaderAwareResponse<T>
where
    T::StringResult: Serialize,
{
    fn fallback(self) -> AxumResponse {
        if self
            .1
            .contains_key(HeaderName::from_static("x-stringify-snowflakes"))
        {
            Response(self.0 .0, self.0 .1.into_string_ids()).into_response()
        } else {
            self.0.into_response()
        }
    }
}

impl<T: Serialize + CastSnowflakes> IntoResponse for HeaderAwareResponse<T>
where
    T::StringResult: Serialize,
{
    fn into_response(self) -> AxumResponse {
        if let Some(accept) = self.1.get(ACCEPT) {
            let value = match accept.to_str() {
                Ok(value) => value,
                Err(_) => {
                    return self.fallback();
                }
            };

            if value == "application/msgpack" || value == "application/x-msgpack" {
                // TODO: ease limitation of only allowing integer snowflakes for msgpack
                match rmp_serde::to_vec(&self.0 .1) {
                    Ok(bytes) => axum::http::Response::builder()
                        .status(self.0 .0)
                        .header(CONTENT_TYPE, "application/msgpack")
                        .body(axum::body::Full::from(bytes))
                        .expect("invalid http status code received")
                        .into_response(),
                    Err(err) => serialization_error(&err),
                }
            } else {
                self.fallback()
            }
        } else {
            self.fallback()
        }
    }
}
