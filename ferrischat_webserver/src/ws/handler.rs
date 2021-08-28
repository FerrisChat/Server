use actix::{Actor, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws::{self, CloseCode, CloseReason, Message, ProtocolError};
use bytes::Bytes;
use ferrischat_common::ws::WsEvent;
use ferrischat_redis::{redis, REDIS_MANAGER};
use serde_json::error::Category;

pub async fn ws_connect(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(WsHandler, &req, stream)
}

pub struct WsHandler;
impl Actor for WsHandler {
    type Context = ws::WebsocketContext<Self>;
}

enum WsEventType {
    Text(String),
    Ping(Bytes),
    Pong(Bytes),
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsHandler {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        let s = match msg {
            Ok(msg) => match msg {
                Message::Text(d) => WsEventType::Text(d.to_string()),
                Message::Binary(_) => {
                    ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
                    return;
                }
                Message::Continuation(_) => {
                    ctx.close(Some(CloseReason::from(CloseCode::Unsupported)));
                    return;
                }
                Message::Ping(m) => WsEventType::Ping(m),
                Message::Pong(m) => WsEventType::Pong(m),
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
        };

        let mut redis_conn = match REDIS_MANAGER.get() {
            Some(r) => r.clone(), // safe to clone cheaply according to docs
            None => {
                ctx.close(Some(CloseReason::from((
                    CloseCode::Error,
                    "redis pool not found",
                ))));
                return;
            }
        };

        let s = match s {
            WsEventType::Text(s) => s,
            WsEventType::Ping(_) => {
                /*
                match redis::cmd("SET")
                    .arg(&[format!("ws:{}:ping", conn_id), "1"])
                    .arg(&["EX", "60"])
                    .query::<String>(&mut redis_conn)
                {
                    Ok(mut r) => {
                        r.make_ascii_lowercase();
                        if !r.contains("ok") {
                            ctx.close(Some(CloseReason::from((
                                CloseCode::Error,
                                "redis returned a error",
                            ))))
                        }
                    }
                    Err(e) => ctx.close(Some(CloseReason::from((
                        CloseCode::Error,
                        format!("redis errored: {}", e),
                    )))),
                };
                */
                ctx.pong(b"{\"code\": 10}");
                return;
            }
            WsEventType::Pong(_) => {
                /*
                redis::cmd("SET")
                    .arg(&[format!("ws:{}:ping", conn_id), "1"])
                    .arg(&["EX", "60"])
                    .query(&mut redis_conn);
                */
                ctx.ping(b"{\"code\": 9}");
                return;
            }
        };

        /*
        let r = match serde_json::from_str::<WsEvent>() {
            Ok(r) => r,
            Err(e) => {
                ctx.close(Some(CloseReason::from((
                    CloseCode::from(2000),
                    format!(
                        "JSON error ({}) at line {}, char {}",
                        match e.classify() {
                            Category::Io => "IO error",
                            Category::Syntax => "syntax error",
                            Category::Data => "invalid data",
                            Category::Eof => "end of file",
                        },
                        e.line(),
                        e.column(),
                    ),
                ))));
                return;
            }
        };

        match r {
            WsEvent::Identify { token, intents } => {}
        }
         */
    }
}
