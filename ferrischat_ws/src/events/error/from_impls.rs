use super::WebSocketHandlerError;
use ferrischat_auth::{SplitTokenError, VerifyTokenFailure};
use tokio::sync::mpsc::error::SendError;

impl From<sqlx::Error> for WebSocketHandlerError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

impl From<VerifyTokenFailure> for WebSocketHandlerError {
    fn from(e: VerifyTokenFailure) -> Self {
        Self::TokenVerifyFail(e)
    }
}

impl<T> From<SendError<T>> for WebSocketHandlerError {
    fn from(_: SendError<T>) -> Self {
        Self::ConnectionClosing
    }
}

impl From<ferrischat_auth::SplitTokenError> for WebSocketHandlerError {
    fn from(e: SplitTokenError) -> Self {
        Self::SplitTokenFail(e)
    }
}
