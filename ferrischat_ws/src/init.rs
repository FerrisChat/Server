#![allow(clippy::module_name_repetitions)]
use crate::handle_connection::handle_ws_connection;
use crate::redis_handler::redis_event_handler;
use crate::{SUB_TO_ME, USERID_CONNECTION_MAP};
use dashmap::DashMap;
use ferrischat_redis::get_pubsub;
use tokio::sync::oneshot::channel;

/// Initialize the `WebSocket` by starting all services it depends on.
///
/// # Panics
/// This function panics if it is called more than once.
pub async fn init_ws() {
    // plop the DashMap into the UserId connection map first thing
    USERID_CONNECTION_MAP
        .set(DashMap::new())
        .unwrap_or_else(|_| panic!("don't call `preload_ws()` more than once"));

    // allow up to 250 new subscriptions to be processed
    let (tx, rx) = tokio::sync::mpsc::channel(250);

    SUB_TO_ME
        .set(tx)
        .expect("don't call `preload_ws()` more than once");

    tokio::spawn(redis_event_handler(
        get_pubsub()
            .await
            .expect("failed to open pubsub connection"),
        rx,
    ));
}

#[allow(clippy::missing_panics_doc)]
/// Initialize the `WebSocket` server.
/// `init_ws` MUST be called before this, otherwise panics may occur due to missing dependencies.
pub async fn init_ws_server() {
    enum DieOrResult {
        Die,
        Result(std::io::Result<tokio::net::UnixStream>),
    }

    let listener = tokio::net::UnixListener::bind(format!(
        "{}/websocket.sock",
        std::env::var("FERRISCHAT_HOME").unwrap_or("/etc/ferrischat/".to_string())
    ))
    .expect("failed to bind to socket!");

    let (end_tx, mut end_rx) = channel();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl+c");
        end_tx
            .send(())
            .expect("failed to send message to listeners");
    });

    tokio::spawn(async move {
        loop {
            let res = tokio::select! {
                stream_addr = listener.accept() => {DieOrResult::Result(stream_addr.map(|x| x.0))}
                _ = &mut end_rx => {DieOrResult::Die}
            };

            match res {
                DieOrResult::Die => break,
                DieOrResult::Result(r) => match r {
                    Ok(stream) => {
                        tokio::spawn(handle_ws_connection(stream));
                    }
                    Err(e) => error!("failed to accept WS conn: {}", e),
                },
            }
        }
    });
}
