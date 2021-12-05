use axum::http::Response;
use axum::response::IntoResponse;
use ferrischat_common::types::ErrorJson;
use ferrischat_redis::deadpool_redis::redis::RedisError;
use ferrischat_redis::deadpool_redis::PoolError;
use sqlx::Error;
use std::borrow::Cow;

pub enum WebServerError {
    Database(sqlx::Error),
    MissingDatabase,
    Json(simd_json::Error),
    Redis(RedisError),
    MissingRedis,
    RedisPool(PoolError),
    Http(ErrorJson),
    RandomGenerationFailure,
    MissingHasher,
    MissingVerifier,
    MissingNodeId,
}

impl From<PoolError> for WebServerError {
    fn from(e: PoolError) -> Self {
        Self::RedisPool(e)
    }
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

impl From<RedisError> for WebServerError {
    fn from(e: RedisError) -> Self {
        Self::Redis(e)
    }
}

impl From<ErrorJson> for WebServerError {
    fn from(e: ErrorJson) -> Self {
        Self::Http(e)
    }
}

impl IntoResponse for WebServerError {
    type Body = axum::body::Body;
    type BodyError = <Self::Body as axum::body::HttpBody>::Error;

    fn into_response(self) -> Response<Self::Body> {
        let body = match self {
            WebServerError::Database(e) => {
                if let sqlx::Error::Database(e) = e {
                    if e.code() == Some(Cow::from("23505")) {
                        ErrorJson::new_409("This object is a duplicate.".to_string())
                    } else {
                        ErrorJson::new_500(format!("Database returned an error: {:?}", e), false, None)
                    }
                } else {
                    ErrorJson::new_500(
                        format!("Database returned an error: {:?}", e),
                        false,
                        None,
                    )
                }
            }
            WebServerError::MissingDatabase => ErrorJson::new_500(
                "Database pool was not found".to_string(),
                false,
                None,
            ),
            WebServerError::Json(e) => ErrorJson::new_500(format!("JSON (de)serialization failed: {}", e), false, None),
            WebServerError::Http(e) => e,
            WebServerError::Redis(e) => ErrorJson::new_500(format!("Redis returned an error: {}", e), false, None),
            WebServerError::MissingRedis => ErrorJson::new_500("Redis pool missing".to_string(), false, None),
            WebServerError::RedisPool(e) => ErrorJson::new_500(format!("Redis pool returned an error: {}", e), false, None),
            WebServerError::RandomGenerationFailure => ErrorJson::new_500(
                "failed to generate random bits for token generation".to_string(),
                true,
                Some(
                    "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                    labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+failed+to+generate+random+bits"
                        .to_string(),
                ),
            ),
            WebServerError::MissingHasher => ErrorJson::new_500(
                "password hasher not found".to_string(),
                false,
                None,
            ),
            WebServerError::MissingVerifier => ErrorJson::new_500(
                "password verifier not found".to_string(),
                true,
                Some(
                    "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+password+verifier+not+found"
                        .to_string(),
                )),

            WebServerError::MissingNodeId => ErrorJson::new_500(
                "Redis has not been set up yet".to_string(),
                true,
                Some(
                    "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&labels=bug&\
                    template=api_bug_report.yml&title=%5B500%5D%3A+redis+not+set+up"
                        .to_string()),
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
            .status(body.get_code())
            .body(axum::body::Body::from(bytes))
            .unwrap_or_else(|e| {
                // this should only be reachable if a invalid HTTP code is passed in
                unreachable!(
                    "got an error while attempting to construct HTTP response for ServerError: {}",
                    e
                )
            })
    }
}
