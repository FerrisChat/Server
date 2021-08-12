use actix::{Actor, StreamHandler};
use actix_web_actors::ws;
use actix_web_actors::ws::{Message, ProtocolError};

pub struct WsHandler;
impl Actor for WsHandler {
    type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsHandler {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                match msg {
                    Message::Text(_) => {}
                    Message::Binary(_) => {}
                    Message::Continuation(_) => {}
                    Message::Ping(m) => ctx.pong(m.as_ref()),
                    Message::Pong(m) => ctx.ping(m.as_ref()),
                    Message::Close(_) => {}
                    Message::Nop => {}
                }
            }
            Err(e) => {}
        }
    }
}
