#![feature(once_cell)]

use dashmap::DashMap;
use ferrischat_auth::{split_token, verify_token};
use ferrischat_common::ws::{WsInboundEvent, WsOutboundEvent};
use ferrischat_redis::{get_pubsub, redis, REDIS_MANAGER};
use futures_util::{SinkExt, StreamExt};
use std::lazy::SyncOnceCell as OnceCell;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpStream;
use tokio_tungstenite::accept_async_with_config;
use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

#[macro_use]
extern crate ferrischat_macros;

/// Maximum number of messages to buffer in the WebSocket send queue.
const MAX_SEND_QUEUE: usize = 32_768;
/// Maximum size of a WebSocket message.
const MAX_MESSAGE_SIZE: usize = 67_108_864; // 64 MiB
/// Maximum size of a single WebSocket frame.
const MAX_FRAME_SIZE: usize = 16_777_216; // 16 MiB

const WEBSOCKET_CONFIG: WebSocketConfig = WebSocketConfig {
    max_send_queue: Some(MAX_SEND_QUEUE),
    max_message_size: Some(MAX_MESSAGE_SIZE),
    max_frame_size: Some(MAX_FRAME_SIZE),
    accept_unmasked_frames: false,
};

enum TxRxComm {
    Ping,
    Pong,
    Text(String),
    Binary(Vec<u8>),
}

// ignore the name
static SUB_TO_ME: OnceCell<
    futures::channel::mpsc::Sender<(String, tokio::sync::mpsc::Sender<Option<redis::Msg>>)>,
> = OnceCell::new();

pub async fn preload_redis() {
    // allow up to 250 new subscriptions to be processed
    let (tx, mut rx) = futures::channel::mpsc::channel(250);

    SUB_TO_ME
        .set(tx)
        .expect("don't call `preload_redis()` more than once");

    let mut pubsub_conn = get_pubsub()
        .await
        .expect("failed to open pubsub connection");

    let local_map: DashMap<String, tokio::sync::mpsc::Sender<Option<redis::Msg>>> = DashMap::new();
    tokio::spawn(async move {
        let mut to_unsub: Vec<String> = Vec::new();
        loop {
            {
                let mut s = pubsub_conn.on_message();
                for _ in 0..150 {
                    if let Some(x) = s.next().await {
                        if let Ok(Some(pat)) = x.get_pattern::<Option<String>>() {
                            if let Some(item) = local_map.get(&pat) {
                                let sender: &tokio::sync::mpsc::Sender<_> = item.value();
                                if let Err(_) = sender.send(Some(x)).await {
                                    to_unsub.push(pat);
                                };
                            }
                        }
                    } else {
                        break; // stream exhausted
                    }
                }
                // drop the stream, losing a &mut ref to it
            }
            // now poll up to 10x for more items in the new subscriptions category
            for _ in 0..10 {
                match rx.try_next() {
                    Ok(Some((channel, output_queue))) => {
                        pubsub_conn.psubscribe(channel.clone()).await;
                        local_map.insert(channel, output_queue);
                    }
                    Ok(None) | Err(_) => break,
                }
            }
            // if there are any, remove nonexistent subscriptions
            for x in &to_unsub {
                pubsub_conn.punsubscribe(x).await;
            }
            // clear the vec
            to_unsub.clear();
        }
    });
}

pub async fn handle_ws_connection(stream: TcpStream, addr: SocketAddr) -> Result<(), Error> {
    let s = accept_async_with_config(stream, Some(WEBSOCKET_CONFIG)).await?;

    let (mut tx, mut rx) = s.split();

    let (inter_tx, mut inter_rx) = futures::channel::mpsc::channel(100);
    let (closer_tx, mut closer_rx) = futures::channel::oneshot::channel::<Option<Error>>();
    let identify_received = AtomicBool::new(false);

    let rx_future = tokio::spawn(async move {
        let mut inter_tx = inter_tx;
        while let Some(item) = rx.next().await {
            let data: Result<ferrischat_common::ws::WsInboundEvent, _> = match item {
                Ok(m) => {
                    match m {
                        Message::Text(t) => simd_json::serde::from_slice(&mut t.into_bytes()[..]),
                        Message::Binary(_) => {
                            // TODO: close WS conn with invalid type
                            closer_tx.send(None);
                            break;
                        }
                        Message::Ping(_) => {
                            inter_tx.send(TxRxComm::Pong).await;
                            continue;
                        }
                        Message::Pong(_) => {
                            inter_tx.send(TxRxComm::Ping).await;
                            continue;
                        }
                        Message::Close(_) => {
                            closer_tx.send(None);
                            break;
                        }
                    }
                }
                Err(e) => {
                    match e {
                        Error::ConnectionClosed => {}
                        Error::AlreadyClosed => {}
                        Error::Io(_) => {}
                        Error::Tls(_) => {}
                        Error::Capacity(_) => {}
                        Error::Protocol(_) => {}
                        Error::Utf8 => {}
                        Error::Url(_) => {}
                        Error::Http(_) => {}
                        Error::HttpFormat(_) => {}
                        _ => unreachable!(),
                    }
                    closer_tx.send(Some(e));
                    break;
                }
            };

            let data = match data {
                Ok(d) => d,
                Err(_) => {
                    // TODO: give reason for closure (ie location of invalid JSON)
                    closer_tx.send(None);
                    break;
                }
            };

            if !identify_received.load(Ordering::Relaxed) {
                match data {
                    WsInboundEvent::Identify { .. } | WsInboundEvent::Resume { .. } => {}
                    _ => {
                        // TODO: give reason for closure (sending data before opened connection)
                        closer_tx.send(None);
                        break;
                    }
                }
            }

            let redis_conn = match REDIS_MANAGER.get() {
                Some(r) => r.clone(), // safe to clone cheaply according to docs
                None => {
                    // TODO: give reason for closure (redis pool went poof)
                    closer_tx.send(None);
                    break;
                }
            };
            let db = match ferrischat_db::DATABASE_POOL.get() {
                Some(db) => db,
                None => {
                    // TODO: give reason for closure (database pool went poof)
                    closer_tx.send(None);
                    break;
                }
            };

            match data {
                WsInboundEvent::Identify { token, intents } => {
                    if identify_received.swap(true, Ordering::Relaxed) {
                        // TODO: give reason for closure (too many IDENTIFY payloads)
                        closer_tx.send(None);
                        break;
                    }

                    let (id, secret) = match split_token(token) {
                        Ok((id, secret)) => (id, secret),
                        Err(_) => {
                            // TODO: give reason for closure (invalid token)
                            closer_tx.send(None);
                            break;
                        }
                    };
                    match verify_token(id, secret).await {
                        Ok(_) => {
                            // token valid
                            let res = sqlx::query!(
                                "SELECT * FROM users WHERE id = $1",
                                u128_to_bigdecimal!(id)
                            )
                            .fetch_one(db)
                            .await;

                            let user = match res {
                                Ok(u) => ferrischat_common::types::User {
                                    id,
                                    name: u.name,
                                    avatar: None,
                                    guilds: None,
                                    flags: u.flags,
                                    discriminator: u.discriminator,
                                },
                                Err(_) => {
                                    // TODO: give reason for closure (internal DB error)
                                    closer_tx.send(None);
                                    break;
                                }
                            };

                            // simd_json doesn't reimplement serialization so this just falls back to `serde_json`
                            let payload = match simd_json::serde::to_string(
                                &WsOutboundEvent::IdentifyAccepted { user },
                            ) {
                                Ok(v) => v,
                                Err(e) => {
                                    // TODO: give reason for closure (failed to serialize JSON)
                                    closer_tx.send(None);
                                    break;
                                }
                            };
                            inter_tx.send(TxRxComm::Text(payload)).await;
                        }
                        Err(_) => {}
                    }
                }
                WsInboundEvent::Resume {
                    token,
                    session_id,
                    sequence,
                } => {}
            }
        }
        rx
    });

    let tx_future = tokio::spawn(async move {
        return (
            loop {
                if let Ok(Some(i)) = closer_rx.try_recv() {
                    break i;
                }

                for _ in 0..100 {
                    match inter_rx.try_next() {
                        Ok(Some(val)) => {
                            match val {
                                TxRxComm::Ping => tx.feed(Message::Pong(
                                    "{\"c\": \"Pong\"}"
                                        .to_string()
                                        .as_bytes()
                                        .iter()
                                        .map(|x| *x)
                                        .collect(),
                                )),
                                TxRxComm::Pong => tx.feed(Message::Ping(
                                    "{\"c\": \"Ping\"}"
                                        .to_string()
                                        .as_bytes()
                                        .iter()
                                        .map(|x| *x)
                                        .collect(),
                                )),
                                TxRxComm::Text(d) => tx.feed(Message::Text(d)),
                                TxRxComm::Binary(d) => tx.feed(Message::Binary(d)),
                            };
                        }
                        Ok(None) => break,
                        Err(_) => {}
                    }
                }
                tx.flush().await;
            },
            tx,
        );
    });

    tokio::spawn(async move {
        let rx = rx_future.await.expect("background rx thread failed");
        let (reason, tx) = tx_future.await.expect("background tx thread failed");

        let stream = rx.reunite(tx).expect("mismatched streams returned");

        // stream.close();
    });

    Ok(())
}

pub async fn init_ws_server<T: tokio::net::ToSocketAddrs>(addr: T) {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind to address");

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    handle_ws_connection(stream, addr).await;
                }
                Err(_) => {}
            }
        }
    });
}
