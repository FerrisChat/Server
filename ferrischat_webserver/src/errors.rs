use axum::body::{self, BoxBody};
use axum::http::Response;
use axum::response::IntoResponse;
use ferrischat_auth::{Argon2Error, SplitTokenError, VerifyTokenFailure};
use ferrischat_common::types::ErrorJson;
use ferrischat_redis::deadpool_redis::redis::RedisError;
use ferrischat_redis::deadpool_redis::PoolError;
use http::header::CONTENT_TYPE;
use http::HeaderValue;
use lettre::address::AddressError;
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

impl From<lettre::address::AddressError> for WebServerError {
    fn from(e: AddressError) -> Self {
        Self::Http(ErrorJson::new_400(format!(
            "failed to parse email address: {}",
            e
        )))
    }
}

impl From<lettre::error::Error> for WebServerError {
    fn from(e: lettre::error::Error) -> Self {
        Self::Http(ErrorJson::new_500(
            format!("Email content error: {}", e),
            false,
            None,
        ))
    }
}

impl From<lettre::transport::smtp::Error> for WebServerError {
    fn from(e: lettre::transport::smtp::Error) -> Self {
        Self::Http(ErrorJson::new_500(
            format!("SMTP transport error: {}", e),
            false,
            None,
        ))
    }
}

impl From<ferrischat_auth::VerifyTokenFailure> for WebServerError {
    fn from(e: VerifyTokenFailure) -> Self {
        let reason = match e {
            VerifyTokenFailure::MissingDatabase => "database pool not found".to_string(),
            VerifyTokenFailure::DbError(e) => return Self::from(e),
            VerifyTokenFailure::VerifierError(e) => {
                format!("argon2 verifier returned an error: {}", e)
            }
            VerifyTokenFailure::InvalidToken => {
                unreachable!("a invalid token error should be handled earlier")
            }
        };
        Self::Http(ErrorJson::new_500(reason, false, None))
    }
}

impl From<Argon2Error> for WebServerError {
    fn from(e: Argon2Error) -> Self {
        let reason = format!(
            "hashing error: {}",
            match e {
                Argon2Error::Communication => {
                    "an error was encountered while waiting for a background thread to complete."
                        .to_string()
                }
                Argon2Error::Argon(e) =>
                    format!("underlying argon2 algorithm threw an error: {}", e),
                Argon2Error::PasswordHash(e) => {
                    format!("password string handling library threw an error: {}", e)
                }
                Argon2Error::MissingConfig => "global configuration unset".to_string(),
                _ => "unknown error".to_string(),
            }
        );
        Self::Http(ErrorJson::new_500(reason, false, None))
    }
}

impl From<SplitTokenError> for WebServerError {
    fn from(e: SplitTokenError) -> Self {
        let message = match e {
            SplitTokenError::InvalidUtf8(e) => format!("invalid utf8 found in token: {}", e),
            SplitTokenError::Base64DecodeError(e) => {
                format!("invalid base64 data found in token: {}", e)
            }
            SplitTokenError::InvalidInteger(e) => format!("invalid integer found in token: {}", e),
            SplitTokenError::MissingParts(idx) => format!("part {} of token missing", idx),
        };
        Self::Http(ErrorJson::new_400(message))
    }
}

impl IntoResponse for WebServerError {
    fn into_response(self) -> Response<BoxBody> {
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
                    .header(CONTENT_TYPE, HeaderValue::from_static("text/plain"))
                    .body(body::boxed(body::Full::from(err.to_string())))
                    .expect("failed to convert static data to a valid request");
            }
        };

        axum::http::Response::builder()
            .status(body.get_code())
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
