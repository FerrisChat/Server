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
use tokio::sync::oneshot::channel;
use tokio_tungstenite::accept_async_with_config;
use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

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

static USERID_CONNECTION_MAP: OnceCell<DashMap<Uuid, u128>> = OnceCell::new();

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
    // plop the DashMap into the UserId connection map first thing
    USERID_CONNECTION_MAP.set(DashMap::new());

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

    let (inter_tx, mut inter_rx) = tokio::sync::mpsc::channel(100);
    let (closer_tx, mut closer_rx) = futures::channel::oneshot::channel::<Option<CloseFrame>>();
    let identify_received = AtomicBool::new(false);
    let conn_id = Uuid::new_v4();

    let rx_future = tokio::spawn(async move {
        let inter_tx = inter_tx;
        while let Some(item) = rx.next().await {
            let data: Result<ferrischat_common::ws::WsInboundEvent, _> = match item {
                Ok(m) => match m {
                    Message::Text(t) => simd_json::serde::from_slice(&mut t.into_bytes()[..]),
                    Message::Binary(_) => {
                        closer_tx.send(Some(CloseFrame {
                            code: CloseCode::Unsupported,
                            reason: "Binary data sent: only text supported at the moment".into(),
                        }));
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
                },
                Err(e) => {
                    let reason = match e {
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
                            reason: format!("I/O error on underlying TCP connection: {}", io)
                                .into(),
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
                    };
                    closer_tx.send(Some(reason));
                    break;
                }
            };

            let data = match data {
                Ok(d) => d,
                Err(e) => {
                    closer_tx.send(Some(CloseFrame {
                        code: CloseCode::from(2001),
                        reason: format!("invalid JSON found: {}", e).into(),
                    }));
                    break;
                }
            };

            if !identify_received.load(Ordering::Relaxed) {
                match data {
                    WsInboundEvent::Identify { .. } | WsInboundEvent::Resume { .. } => {}
                    _ => {
                        closer_tx.send(Some(CloseFrame {
                            code: CloseCode::from(2004),
                            reason: "data payload sent before identifying".into(),
                        }));
                        break;
                    }
                }
            }

            let redis_conn = match REDIS_MANAGER.get() {
                Some(r) => r.clone(), // safe to clone cheaply according to docs
                None => {
                    closer_tx.send(Some(CloseFrame {
                        code: CloseCode::from(5002),
                        reason: "Redis pool not found".into(),
                    }));
                    break;
                }
            };
            let db = match ferrischat_db::DATABASE_POOL.get() {
                Some(db) => db,
                None => {
                    closer_tx.send(Some(CloseFrame {
                        code: CloseCode::from(5003),
                        reason: "Database pool not found".into(),
                    }));
                    break;
                }
            };

            let uid_conn_map = match USERID_CONNECTION_MAP.get() {
                Some(m) => m,
                None => {
                    closer_tx.send(Some(CloseFrame {
                        code: CloseCode::from(5004),
                        reason: "Connection map not found".into(),
                    }));
                    break;
                }
            };

            match data {
                WsInboundEvent::Identify { token, intents } => {
                    if identify_received.swap(true, Ordering::Relaxed) {
                        closer_tx.send(Some(CloseFrame {
                            code: CloseCode::from(2002),
                            reason: "Too many IDENTIFY payloads sent".into(),
                        }));
                        break;
                    }

                    let (id, secret) = match split_token(token) {
                        Ok((id, secret)) => (id, secret),
                        Err(_) => {
                            closer_tx.send(Some(CloseFrame {
                                code: CloseCode::from(2003),
                                reason: "Token invalid".into(),
                            }));
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
                                Err(e) => {
                                    closer_tx.send(Some(CloseFrame {
                                        code: CloseCode::from(5000),
                                        reason: format!("Internal database error: {}", e).into(),
                                    }));
                                    break;
                                }
                            };

                            // simd_json doesn't reimplement serialization so this just falls back to `serde_json`
                            let payload = match simd_json::serde::to_string(
                                &WsOutboundEvent::IdentifyAccepted { user },
                            ) {
                                Ok(v) => v,
                                Err(e) => {
                                    closer_tx.send(Some(CloseFrame {
                                        code: CloseCode::from(5001),
                                        reason: format!("JSON serialization error: {}", e).into(),
                                    }));
                                    break;
                                }
                            };
                            inter_tx.send(TxRxComm::Text(payload)).await;

                            uid_conn_map.insert(conn_id, id);
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
        enum TransmitType<'t> {
            InterComm(Option<TxRxComm>),
            Exit(Option<CloseFrame<'t>>),
            Redis(Option<Option<ferrischat_redis::redis::Msg>>),
        }

        let mut redis_rx: Option<
            tokio::sync::mpsc::Receiver<Option<ferrischat_redis::redis::Msg>>,
        > = None;

        let ret = loop {
            let x = match &mut redis_rx {
                Some(rx) => tokio::select! {
                    item = &mut closer_rx => TransmitType::Exit(item.ok().flatten()),
                    item = inter_rx.recv() => TransmitType::InterComm(item),
                    item = rx.recv() => TransmitType::Redis(item),
                },
                None => tokio::select! {
                    item = &mut closer_rx => TransmitType::Exit(item.ok().flatten()),
                    item = inter_rx.recv() => TransmitType::InterComm(item),
                },
            };

            match x {
                TransmitType::InterComm(event) => match event {
                    Some(val) => {
                        match val {
                            TxRxComm::Ping => {
                                tx.feed(Message::Pong(
                                    "{\"c\": \"Pong\"}"
                                        .to_string()
                                        .as_bytes()
                                        .iter()
                                        .map(|x| *x)
                                        .collect(),
                                ))
                                .await
                            }
                            TxRxComm::Pong => {
                                tx.feed(Message::Ping(
                                    "{\"c\": \"Ping\"}"
                                        .to_string()
                                        .as_bytes()
                                        .iter()
                                        .map(|x| *x)
                                        .collect(),
                                ))
                                .await
                            }
                            TxRxComm::Text(d) => tx.feed(Message::Text(d)).await,
                            // the implementation is here
                            // is it used? no
                            TxRxComm::Binary(d) => tx.feed(Message::Binary(d)).await,
                        };
                    }
                    None => break None,
                },
                TransmitType::Exit(reason) => break reason,
                TransmitType::Redis(msg) => {}
            }
            tx.flush().await;

            if redis_rx.is_none() {
                let uid_conn_map = match USERID_CONNECTION_MAP.get() {
                    Some(m) => m,
                    None => {
                        return (
                            Some(CloseFrame {
                                code: CloseCode::from(5004),
                                reason: "Connection map not found".into(),
                            }),
                            tx,
                        );
                    }
                };
                if let Some(map_val) = uid_conn_map.get(&conn_id) {
                    let redis_comm = tokio::sync::mpsc::channel(250);
                    redis_rx = Some(redis_comm.1);
                    match crate::SUB_TO_ME.get() {
                        Some(s) => {
                            let mut s = s.clone();
                            s.start_send((format!("*{}*", *(map_val.value())), redis_comm.0))
                        }
                        None => {
                            // since we drop the sender that was moved in, the other thread will panic
                            return (
                                Some(CloseFrame {
                                    code: CloseCode::from(5002),
                                    reason: "Redis pool not found".into(),
                                }),
                                tx,
                            );
                        }
                    };
                };
            }
        };

        tx.flush().await;

        return (ret, tx);
    });

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
            reason: Default::default(),
        });
        stream.close(Some(f)).await;
    });

    Ok(())
}

pub async fn init_ws_server<T: tokio::net::ToSocketAddrs>(addr: T) {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind to address");

    let (end_tx, mut end_rx) = channel();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await;
        end_tx.send(())
    });

    enum DieOrResult<T> {
        Die,
        Result(tokio::io::Result<T>),
    }

    tokio::spawn(async move {
        loop {
            let res = tokio::select! {
                stream_addr = listener.accept() => {DieOrResult::Result(stream_addr)}
                _ = &mut end_rx => {DieOrResult::Die}
            };

            match res {
                DieOrResult::Die => break,
                DieOrResult::Result(r) => match r {
                    Ok((stream, addr)) => {
                        tokio::spawn(handle_ws_connection(stream, addr));
                    }
                    Err(e) => eprintln!("failed to accept WS conn: {}", e),
                },
            }
        }
    });
}
