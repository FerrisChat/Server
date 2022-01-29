use ferrischat_auth::{SplitTokenError, VerifyTokenFailure};
use std::borrow::Cow;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

mod from_impls;

#[allow(dead_code)]
pub enum WebSocketHandlerError {
    Database(sqlx::Error),
    TooManyIdentify,
    TokenVerifyFail(ferrischat_auth::VerifyTokenFailure),
    BigDecimalToU128Fail,
    ConnectionClosing,
    SplitTokenFail(ferrischat_auth::SplitTokenError),
}

#[allow(clippy::from_over_into)]
impl Into<CloseFrame<'_>> for WebSocketHandlerError {
    fn into(self) -> CloseFrame<'static> {
        match self {
            WebSocketHandlerError::Database(e) => CloseFrame {
                code: CloseCode::from(5000),
                reason: format!("database returned an error: {}", e).into(),
            },
            WebSocketHandlerError::TooManyIdentify => CloseFrame {
                code: CloseCode::from(2002),
                reason: "too many IDENTIFY payloads sent".into(),
            },
            WebSocketHandlerError::TokenVerifyFail(e) => CloseFrame {
                code: CloseCode::from(2003),
                reason: format!(
                    "failed to verify token: {:?}",
                    match e {
                        VerifyTokenFailure::MissingDatabase => "database pool not found".into(),
                        VerifyTokenFailure::DbError(e) =>
                            format!("database returned an error: {}", e).into(),
                        VerifyTokenFailure::VerifierError(e) => {
                            format!("argon2 verifier returned an error: {}", e).into()
                        }
                        VerifyTokenFailure::InvalidToken => {
                            Cow::from("token invalid")
                        }
                    }
                )
                .into(),
            },
            WebSocketHandlerError::BigDecimalToU128Fail => CloseFrame {
                code: CloseCode::from(5006),
                reason: "failed to convert BigDecimal to u128".into(),
            },
            WebSocketHandlerError::ConnectionClosing => CloseFrame {
                code: CloseCode::Normal,
                reason: "normal closure".into(),
            },
            WebSocketHandlerError::SplitTokenFail(e) => CloseFrame {
                code: CloseCode::from(2003),
                reason: format!(
                    "failed to verify token: {:?}",
                    match e {
                        SplitTokenError::InvalidUtf8(e) =>
                            format!("invalid utf8 found in token: {}", e),
                        SplitTokenError::Base64DecodeError(e) => {
                            format!("invalid base64 data found in token: {}", e)
                        }
                        SplitTokenError::InvalidInteger(e) =>
                            format!("invalid integer found in token: {}", e),
                        SplitTokenError::MissingParts(idx) =>
                            format!("part {} of token missing", idx),
                    }
                )
                .into(),
            },
        }
    }
}
