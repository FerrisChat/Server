use ferrischat_auth::{SplitTokenError, VerifyTokenFailure};
use std::borrow::Cow;
use tokio::sync::mpsc::error::SendError;
use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub enum WsEventHandlerError<'a> {
    CloseFrame(CloseFrame<'a>),
    Sender,
}

impl From<VerifyTokenFailure> for WsEventHandlerError<'_> {
    fn from(e: VerifyTokenFailure) -> Self {
        let (code, msg) = match e {
            VerifyTokenFailure::MissingDatabase => (5003, Cow::from("Database pool missing")),
            VerifyTokenFailure::InvalidToken => (2003, Cow::from("Invalid token")),
            VerifyTokenFailure::DbError(e) => (
                5000,
                Cow::from(format!("Database returned an error: {:?}", e)),
            ),
            VerifyTokenFailure::VerifierError(e) => (
                5007,
                Cow::from(format!("Password verifier returned an error: {}", e)),
            ),
        };
        Self::CloseFrame(CloseFrame {
            code: CloseCode::from(code),
            reason: msg,
        })
    }
}

impl From<SplitTokenError> for WsEventHandlerError<'_> {
    fn from(e: SplitTokenError) -> Self {
        let msg = match e {
            SplitTokenError::InvalidUtf8(e) => format!("invalid utf-8 found in token: {}", e),
            SplitTokenError::Base64DecodeError(e) => {
                format!("invalid base64 found in token: {}", e)
            }
            SplitTokenError::InvalidInteger(e) => format!("invalid integer found in token: {}", e),
            SplitTokenError::MissingParts(e) => format!("part {} of token missing", e),
        };
        Self::CloseFrame(CloseFrame {
            code: CloseCode::from(2003),
            reason: format!("Token invalid: {}", msg).into(),
        })
    }
}

impl From<sqlx::Error> for WsEventHandlerError<'_> {
    fn from(e: sqlx::Error) -> Self {
        WsEventHandlerError::CloseFrame(CloseFrame {
            code: CloseCode::from(5000),
            reason: format!("Internal database error: {}", e).into(),
        })
    }
}

impl<T> From<&tokio::sync::mpsc::error::SendError<T>> for WsEventHandlerError<'_> {
    fn from(_: &SendError<T>) -> Self {
        Self::Sender
    }
}

pub fn handle_error<'a>(e: Error) -> CloseFrame<'a> {
    match e {
        Error::ConnectionClosed => CloseFrame {
            code: CloseCode::Normal,
            reason: "connection closed normally".into(),
        },
        Error::AlreadyClosed => CloseFrame {
            code: CloseCode::Normal,
            reason: "connection already closed".into(),
        },
        Error::Io(io) => CloseFrame {
            code: CloseCode::from(1014),
            reason: format!("I/O error on underlying TCP connection: {}", io).into(),
        },
        Error::Tls(tls) => CloseFrame {
            code: CloseCode::from(1015),
            reason: format!("TLS error: {:?}", tls).into(),
        },
        Error::Capacity(cap) => CloseFrame {
            code: CloseCode::from(1016),
            reason: format!("Capacity error: {:?}", cap).into(),
        },
        Error::Protocol(proto) => CloseFrame {
            code: CloseCode::Protocol,
            reason: format!("Protocol error: {:?}", proto).into(),
        },
        Error::Utf8 => CloseFrame {
            code: CloseCode::Invalid,
            reason: "UTF-8 encoding error".into(),
        },
        Error::Url(url) => CloseFrame {
            code: CloseCode::from(1017),
            reason: format!("Invalid URL: {:?}", url).into(),
        },
        Error::Http(http) => CloseFrame {
            code: CloseCode::from(1018),
            reason: format!("HTTP error: {:?}", http).into(),
        },
        Error::HttpFormat(fmt) => CloseFrame {
            code: CloseCode::from(1019),
            reason: format!("HTTP format error: {:?}", fmt).into(),
        },
        Error::SendQueueFull(_) => unreachable!(),
    }
}
