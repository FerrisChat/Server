use futures_util::stream::SplitSink;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

pub type WsTransmit = SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>;
