use axum::http::Response;
use axum::response::IntoResponse;
use ferrischat_common::types::InternalServerErrorJson;
use ferrischat_redis::deadpool_redis::redis::RedisError;
use serde::Serialize;
use sqlx::Error;

pub enum WebServerError<T: Serialize> {
    Database(sqlx::Error),
    Json(simd_json::Error),
    Redis(ferrischat_redis::redis::RedisError),
    MissingRedis,
    RedisPool(ferrischat_redis::deadpool_redis::PoolError),
    Http { code: u16, body: T },
}

impl<T: Serialize> From<sqlx::Error> for WebServerError<T> {
    fn from(e: Error) -> Self {
        Self::Database(e)
    }
}

impl<T: Serialize> From<simd_json::Error> for WebServerError<T> {
    fn from(e: simd_json::Error) -> Self {
        Self::Json(e)
    }
}

impl<T: Serialize> From<ferrischat_redis::redis::RedisError> for WebServerError<T> {
    fn from(e: RedisError) -> Self {
        Self::Redis(e)
    }
}

impl<T: Serialize> From<(u16, T)> for WebServerError<T> {
    fn from((code, obj): (u16, T)) -> Self {
        if !(100 <= code && code <= 599) {
            panic!("invalid HTTP code passed to .from() for WebServerError (valid codes are 100 <= code <= 599)");
        }
        Self { code, obj }
    }
}

impl<T: Serialize> IntoResponse for WebServerError<T> {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        let (code, body) = match self {
            WebServerError::Database(e) => (
                500,
                InternalServerErrorJson {
                    reason: format!("Database returned an error: {:?}", e),
                    is_bug: false,
                    link: None,
                },
            ),
            WebServerError::Json(e) => (
                500,
                InternalServerErrorJson {
                    reason: format!("JSON (de)serialization failed: {}", e),
                    is_bug: false,
                    link: None,
                },
            ),
            WebServerError::Http { code, body } => (code, body),
            WebServerError::Redis(e) => (
                500,
                InternalServerErrorJson {
                    reason: format!("Redis returned an error: {}", e),
                    is_bug: false,
                    link: None,
                },
            ),
            WebServerError::MissingRedis => (
                500,
                InternalServerErrorJson {
                    reason: "Redis pool missing".to_string(),
                    is_bug: false,
                    link: None,
                },
            ),
            WebServerError::RedisPool(e) => (
                500,
                InternalServerErrorJson {
                    reason: format!("Redis pool returned an error: {}", e),
                    is_bug: false,
                    link: None,
                },
            ),
        };

        let bytes = match simd_json::to_vec(&body) {
            Ok(res) => res,
            Err(err) => {
                return Response::builder()
                    .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                    .header(http::header::CONTENT_TYPE, "text/plain")
                    .body(axum::body::Body::from(err.to_string()))
                    .expect("failed to convert static data to a valid request");
            }
        };

        axum::http::Response::builder()
            .status(code)
            .body(axum::body::Body::from(bytes))
            .unwrap_or_else(|e| {
                // this should only be reachable if a invalid HTTP code is passed in
                unreachable!(
                    "got an error while attempting to construct HTTP response for 500: {}",
                    e
                )
            })
    }
}
