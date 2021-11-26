use crate::ws::WsEventError;
use axum::http::Response;
use axum::response::IntoResponse;
use ferrischat_redis::deadpool_redis::redis::RedisError;
use sqlx::Error;

pub enum WebServerError {
    Database(sqlx::Error),
    Json(simd_json::Error),
    Redis(ferrischat_redis::redis::RedisError),
    MissingRedis,
    RedisPool(ferrischat_redis::deadpool_redis::PoolError),
    Http { code: u16, body: String },
}

impl From<sqlx::Error> for WebServerError {
    fn from(e: Error) -> Self {
        Self::Database(e)
    }
}

impl From<simd_json::Error> for WebServerError {
    fn from(e: simd_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<ferrischat_redis::redis::RedisError> for WebServerError {
    fn from(e: RedisError) -> Self {
        Self::Redis(e)
    }
}

impl From<(u16, String)> for WebServerError {
    fn from((code, body): (u16, String)) -> Self {
        if !(100 <= code && code <= 599) {
            panic!("invalid HTTP code passed to .from() for WebServerError (valid codes are 100 <= code <= 599)");
        }
        Self::Http { code, body }
    }
}

impl IntoResponse for WebServerError {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        let (code, body) = match self {
            WebServerError::Database(e) => (500, format!("Database returned an error: {:?}", e)),
            WebServerError::Json(e) => (500, format!("JSON (de)serialization failed: {}", e)),
            WebServerError::Http { code, body } => (code, body),
            WebServerError::Redis(e) => (500, format!("Redis returned an error: {}", e)),
            WebServerError::MissingRedis => (500, "Redis pool missing".to_string()),
            WebServerError::RedisPool(e) => (500, format!("Redis pool returned an error: {}", e)),
        };
        axum::http::Response::builder()
            .status(code)
            .body(axum::body::Body::from(body))
            .unwrap_or_else(|e| {
                // this should only be reachable if a invalid HTTP code is passed in
                unreachable!(
                    "got an error while attempting to construct HTTP response for 500: {}",
                    e
                )
            })
    }
}
