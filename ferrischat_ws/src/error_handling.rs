use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub enum WsEventHandlerError<'a> {
    CloseFrame(CloseFrame<'a>),
    Sender,
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
        _ => unreachable!(),
    }
}
