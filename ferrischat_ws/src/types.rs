use futures_util::stream::SplitSink;
use tokio::net::UnixStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

pub type WsTransmit = SplitSink<WebSocketStream<UnixStream>, Message>;
