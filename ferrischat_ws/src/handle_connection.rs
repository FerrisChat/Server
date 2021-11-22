use crate::config::WEBSOCKET_CONFIG;
use crate::rx_handler::rx_handler;
use crate::tx_handler::tx_handler;
use crate::USERID_CONNECTION_MAP;
use futures_util::StreamExt;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::accept_async_with_config;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Error;
use uuid::Uuid;

pub async fn handle_ws_connection(
    stream: TlsStream<TcpStream>,
    _: SocketAddr,
) -> Result<(), Error> {
    let s = accept_async_with_config(stream, Some(WEBSOCKET_CONFIG)).await?;

    let (tx, rx) = s.split();

    let (inter_tx, inter_rx) = tokio::sync::mpsc::channel(100);
    let (closer_tx, closer_rx) = futures::channel::oneshot::channel();
    let conn_id = Uuid::new_v4();

    let rx_future = tokio::spawn(rx_handler(rx, inter_tx, closer_tx, conn_id));

    let tx_future = tokio::spawn(tx_handler(tx, closer_rx, inter_rx, conn_id));

    tokio::spawn(async move {
        let rx = rx_future.await.expect("background rx thread failed");
        let (reason, tx) = tx_future.await.expect("background tx thread failed");

        let uid_conn_map = USERID_CONNECTION_MAP
            .get()
            .expect("user ID connection map not set");
        uid_conn_map.remove(&conn_id);

        let mut stream = rx.reunite(tx).expect("mismatched streams returned");

        let f = reason.unwrap_or(CloseFrame {
            code: CloseCode::Abnormal,
            reason: std::borrow::Cow::default(),
        });
        stream.close(Some(f)).await.expect("failed to close stream");
    });

    Ok(())
}
