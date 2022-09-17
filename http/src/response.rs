use axum::{
    http::{
        header::{HeaderMap, HeaderName, ACCEPT, CONTENT_TYPE},
        StatusCode,
    },
    response::{IntoResponse, Response as AxumResponse},
};
use common::CastSnowflakes;
use serde::Serialize;

pub use common::http::Error;

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
    #[must_use]
    pub fn promote(self, headers: &HeaderMap) -> HeaderAwareResponse<T> {
        HeaderAwareResponse {
            response: self,
            headers: headers.clone(),
        }
    }

    /// Wraps a promoted version of this response with Ok.
    pub fn promote_ok<E>(self, headers: &HeaderMap) -> Result<HeaderAwareResponse<T>, E> {
        Ok(self.promote(headers))
    }

    /// Wraps a promoted version of this response with Err.
    pub fn promote_err<U>(self, headers: &HeaderMap) -> Result<U, HeaderAwareResponse<T>> {
        Err(self.promote(headers))
    }
}

impl Response<Error> {
    /// Creates a new not found response (404 Not Found) with the given entity.
    #[must_use]
    pub const fn not_found(entity: &'static str, message: String) -> Self {
        Self(
            StatusCode::NOT_FOUND,
            Error::<u128>::NotFound { entity, message },
        )
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
            Error::<u128>::InternalError {
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
            Error::<u128>::InternalError {
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
            Error::<u128>::InternalError {
                what: Some("database"),
                message: err.to_string(),
                debug: Some(format!("{:?}", err)),
            },
        )
    }
}

impl From<argon2_async::Error> for Response<Error> {
    fn from(err: argon2_async::Error) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Error::<u128>::InternalError {
                what: Some("hasher"),
                message: err.to_string(),
                debug: Some(format!("{:?}", err)),
            },
        )
    }
}

impl From<redis::RedisError> for Response<Error> {
    fn from(err: redis::RedisError) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Error::<u128>::InternalError {
                what: Some("redis"),
                message: err.to_string(),
                debug: Some(format!("{:?}", err)),
            },
        )
    }
}

impl From<deadpool_redis::PoolError> for Response<Error> {
    fn from(err: deadpool_redis::PoolError) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Error::<u128>::InternalError {
                what: Some("redis pool"),
                message: err.to_string(),
                debug: Some(format!("{:?}", err)),
            },
        )
    }
}

fn serialization_error(err: &(impl ToString + std::fmt::Debug)) -> AxumResponse {
    Response(
        StatusCode::INTERNAL_SERVER_ERROR,
        Error::<u128>::InternalError {
            what: Some("serialization"),
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
pub struct HeaderAwareResponse<T: Serialize> {
    /// The response.
    pub response: Response<T>,
    /// The request headers.
    pub headers: HeaderMap,
}

impl<T: Serialize> HeaderAwareResponse<T> {
    fn stringify_snowflakes(&self) -> bool {
        self.headers
            .get(HeaderName::from_static("x-stringify-snowflakes"))
            .is_some_and(|value| {
                value
                    .to_str()
                    .is_ok_and(|value| value.to_ascii_lowercase() != "false")
            })
    }

    fn msgpack(&self) -> bool {
        self.headers.get(ACCEPT).is_some_and(|accept| {
            accept.to_str().is_ok_and(|&value| {
                value == "application/msgpack" || value == "application/x-msgpack"
            })
        })
    }

    fn msgpack_compact(&self) -> bool {
        self.headers
            .get(HeaderName::from_static("x-msgpack-compact"))
            .is_some_and(|value| {
                value
                    .to_str()
                    .is_ok_and(|value| value.to_ascii_lowercase() != "false")
            })
    }
}

impl<T: Serialize + CastSnowflakes> HeaderAwareResponse<T>
where
    T::StringResult: Serialize,
{
    fn fallback(self) -> AxumResponse {
        if self.stringify_snowflakes() {
            Response(self.response.0, self.response.1.into_string_ids()).into_response()
        } else {
            self.response.into_response()
        }
    }
}

impl<T: Serialize + CastSnowflakes> IntoResponse for HeaderAwareResponse<T>
where
    T::StringResult: Serialize,
{
    fn into_response(self) -> AxumResponse {
        if self.msgpack() {
            match if self.stringify_snowflakes() {
                if self.msgpack_compact() {
                    rmp_serde::to_vec(&self.response.1.into_string_ids())
                } else {
                    rmp_serde::to_vec_named(&self.response.1.into_string_ids())
                }
            } else {
                let target = &self.response.1;

                if self.msgpack_compact() {
                    rmp_serde::to_vec(target)
                } else {
                    rmp_serde::to_vec_named(target)
                }
            } {
                Ok(bytes) => axum::http::Response::builder()
                    .status(self.response.0)
                    .header(CONTENT_TYPE, "application/msgpack")
                    .body(axum::body::Full::from(bytes))
                    .expect("invalid http status code received")
                    .into_response(),
                Err(err) => serialization_error(&err),
            }
        } else {
            self.fallback()
        }
    }
}

pub trait PromoteErr<T, E> {
    fn promote(self, headers: &HeaderMap) -> Result<T, HeaderAwareResponse<Error>>
    where
        Self: Sized;
}

impl<T, E: Into<Response<Error>>> PromoteErr<T, E> for Result<T, E> {
    fn promote(self, headers: &HeaderMap) -> Result<T, HeaderAwareResponse<Error>> {
        self.map_err(|err| err.into().promote(headers))
    }
}
