use crate::{TxRxComm, USERID_CONNECTION_MAP};
use ferrischat_auth::{split_token, verify_token};
use ferrischat_common::ws::{WsInboundEvent, WsOutboundEvent};
use ferrischat_redis::REDIS_MANAGER;
use futures_util::stream::SplitStream;
use futures_util::StreamExt;
use num_traits::ToPrimitive;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

pub async fn rx_handler(
    mut rx: SplitStream<WebSocketStream<TlsStream<TcpStream>>>,
    inter_tx: tokio::sync::mpsc::Sender<TxRxComm>,
    closer_tx: futures::channel::oneshot::Sender<Option<CloseFrame<'_>>>,
    conn_id: Uuid,
) -> SplitStream<WebSocketStream<TlsStream<TcpStream>>> {
    let identify_received = AtomicBool::new(false);
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
                Message::Ping(_) | Message::Pong(_) => continue,
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
                WsInboundEvent::Identify { .. } => {}
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
                        let bigdecimal_user_id = u128_to_bigdecimal!(id);

                        let res =
                            sqlx::query!("SELECT * FROM users WHERE id = $1", bigdecimal_user_id)
                                .fetch_one(db)
                                .await;

                        let user = match res {
                            Ok(u) => ferrischat_common::types::User {
                                id,
                                name: u.name,
                                avatar: None,
                                guilds: {
                                    let resp = sqlx::query!(
                                            r#"SELECT id AS "id!", owner_id AS "owner_id!", name AS "name!" FROM guilds INNER JOIN members m on guilds.id = m.guild_id WHERE m.user_id = $1"#,
                                            bigdecimal_user_id
                                        )
                                        .fetch_all(db)
                                        .await;

                                    match resp {
                                        Ok(d) => {
                                            let mut guilds = Vec::with_capacity(d.len());
                                            for x in d {
                                                let id_ =
                                                    x.id.clone()
                                                        .with_scale(0)
                                                        .into_bigint_and_exponent()
                                                        .0
                                                        .to_u128();

                                                let id = match id_ {
                                                    Some(id) => id,
                                                    None => continue,
                                                };

                                                let owner_id_ = x
                                                    .owner_id
                                                    .with_scale(0)
                                                    .into_bigint_and_exponent()
                                                    .0
                                                    .to_u128();

                                                let owner_id = match owner_id_ {
                                                    Some(owner_id) => owner_id,
                                                    None => continue,
                                                };

                                                let g = ferrischat_common::types::Guild {
                                                    id,
                                                    owner_id,
                                                    name: x.name.clone(),
                                                    channels: {
                                                        let resp = sqlx::query!(
                                                                    "SELECT * FROM channels WHERE guild_id = $1",
                                                                    x.id.clone()
                                                                )
                                                            .fetch_all(db)
                                                            .await;

                                                        Some(match resp {
                                                            Ok(resp) => resp
                                                                .iter()
                                                                .filter_map(|x| {
                                                                    Some(ferrischat_common::types::Channel {
                                                                        id: x.id.clone().with_scale(0).into_bigint_and_exponent().0.to_u128()?,
                                                                        name: x.name.clone(),
                                                                        guild_id: x
                                                                            .guild_id
                                                                            .with_scale(0)
                                                                            .into_bigint_and_exponent()
                                                                            .0
                                                                            .to_u128()?,
                                                                    })
                                                                })
                                                                .collect(),
                                                            Err(e) => {
                                                                continue; // TODO: zero shall fix when refactor
                                                            }
                                                        })
                                                    },
                                                    flags:
                                                        ferrischat_common::types::GuildFlags::empty(
                                                        ),
                                                    members: {
                                                        let resp = sqlx::query!(
                                                                    "SELECT m.*, u.name AS name, u.discriminator AS discriminator, u.flags AS flags FROM members m CROSS JOIN LATERAL (SELECT * FROM users u WHERE id = m.user_id) AS u WHERE guild_id = $1",
                                                                    x.id.clone()
                                                                )
                                                            .fetch_all(db)
                                                            .await;

                                                        Some(match resp {
                                                            Ok(resp) => resp
                                                                .iter()
                                                                .filter_map(|x| {
                                                                    let user_id = x.user_id
                                                                                   .with_scale(0)
                                                                                   .into_bigint_and_exponent()
                                                                                   .0
                                                                                   .to_u128()?;
                                                                    Some(ferrischat_common::types::Member {
                                                                        user_id: Some(user_id),
                                                                        user: Some(ferrischat_common::types::User {
                                                                            id: user_id,
                                                                            name: x.name.clone(),
                                                                            avatar: None,
                                                                            guilds: None,
                                                                            flags: ferrischat_common::types::UserFlags::from_bits_truncate(x.flags),
                                                                            discriminator: x.discriminator,
                                                                        }),
                                                                        guild_id: x.guild_id.with_scale(0).into_bigint_and_exponent().0.to_u128(),
                                                                        guild: None,
                                                                    })
                                                                })
                                                                .collect(),
                                                            Err(e) => {
                                                                continue; // TODO: zero shall fix when refactor
                                                            }
                                                        })
                                                    },
                                                    roles: None,
                                                };
                                                guilds.push(g);
                                            }
                                            Some(guilds)
                                        }
                                        Err(e) => {
                                            closer_tx.send(Some(CloseFrame {
                                                code: CloseCode::from(5000),
                                                reason: format!("Internal database error: {}", e)
                                                    .into(),
                                            }));
                                            break;
                                        }
                                    }
                                },
                                flags: ferrischat_common::types::UserFlags::from_bits_truncate(
                                    u.flags,
                                ),
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
                        let payload =
                            match simd_json::serde::to_string(&WsOutboundEvent::IdentifyAccepted {
                                user,
                            }) {
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
            WsInboundEvent::Ping => {
                inter_tx
                    .send(TxRxComm::Text(
                        match simd_json::serde::to_string(&WsOutboundEvent::Pong) {
                            Ok(v) => v,
                            Err(e) => {
                                closer_tx.send(Some(CloseFrame {
                                    code: CloseCode::from(5001),
                                    reason: format!("JSON serialization error: {}", e).into(),
                                }));
                                break;
                            }
                        },
                    ))
                    .await;
            }
            WsInboundEvent::Pong => {
                inter_tx
                    .send(TxRxComm::Text(
                        match simd_json::serde::to_string(&WsOutboundEvent::Ping) {
                            Ok(v) => v,
                            Err(e) => {
                                closer_tx.send(Some(CloseFrame {
                                    code: CloseCode::from(5001),
                                    reason: format!("JSON serialization error: {}", e).into(),
                                }));
                                break;
                            }
                        },
                    ))
                    .await;
            }
        }
    }
    rx
}
