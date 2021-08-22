use actix::{Actor, StreamHandler};
use actix_web_actors::ws::{self, CloseCode, CloseReason, Message, ProtocolError};

pub struct WsHandler;
impl Actor for WsHandler {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsHandler {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => match msg {
                Message::Text(d) => {
                    // TODO: json (de)?serialization
                }
                Message::Binary(_) => {
                    ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
                    return;
                }
                Message::Continuation(_) => {
                    ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
                    return;
                }
                Message::Ping(m) => {
                    ctx.pong(m.as_ref());
                    return;
                }
                Message::Pong(m) => {
                    ctx.ping(m.as_ref());
                    return;
                }
                Message::Close(_) => {
                    ctx.close(Some(CloseReason::from(CloseCode::Normal)));
                    return;
                }
                Message::Nop => {
                    ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
                    return;
                }
            },
            Err(e) => {
                ctx.close(Some(CloseReason::from((
                    CloseCode::Protocol,
                    format!(
                        "websocket protocol error: {}",
                        match e {
                            ProtocolError::UnmaskedFrame =>
                                "client sent unmasked frame".to_string(),
                            ProtocolError::MaskedFrame => "server sent masked frame".to_string(),
                            ProtocolError::InvalidOpcode(c) => {
                                format!("got invalid opcode: {}", c)
                            }
                            ProtocolError::InvalidLength(len) => {
                                format!("got invalid control frame length: {}", len)
                            }
                            ProtocolError::BadOpCode => "got bad opcode".to_string(),
                            ProtocolError::Overflow => "payload reached size limit".to_string(),
                            ProtocolError::ContinuationNotStarted =>
                                "continuation not started".to_string(),
                            ProtocolError::ContinuationStarted =>
                                "received new continuation but it's already started".to_string(),
                            ProtocolError::ContinuationFragment(f) => {
                                format!("unknown continuation fragment: {}", f)
                            }
                            ProtocolError::Io(e) => {
                                format!("IO error: {}", e)
                            }
                        }
                    ),
                ))));
                return;
            }
        }
    }
}
