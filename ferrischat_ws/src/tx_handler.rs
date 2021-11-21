use crate::{TxRxComm, USERID_CONNECTION_MAP};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_redis::redis::Msg;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use num_traits::ToPrimitive;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

pub async fn tx_handler(
    mut tx: SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>,
    mut closer_rx: futures::channel::oneshot::Receiver<Option<CloseFrame<'_>>>,
    mut inter_rx: tokio::sync::mpsc::Receiver<TxRxComm>,
    conn_id: Uuid,
) -> (
    Option<CloseFrame<'_>>,
    SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>,
) {
    enum TransmitType<'t> {
        InterComm(Option<TxRxComm>),
        Exit(Option<CloseFrame<'t>>),
        Redis(Option<Option<Msg>>),
    }

    let mut redis_rx: Option<tokio::sync::mpsc::Receiver<Option<Msg>>> = None;

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

        let db = match ferrischat_db::DATABASE_POOL.get() {
            Some(db) => db,
            None => {
                return (
                    Some(CloseFrame {
                        code: CloseCode::from(5003),
                        reason: "Database pool not found".into(),
                    }),
                    tx,
                );
            }
        };

        let uid_conn_map = match USERID_CONNECTION_MAP.get() {
            Some(m) => m,
            None => {
                return (
                    Some(CloseFrame {
                        code: CloseCode::from(5003),
                        reason: "Database pool not found".into(),
                    }),
                    tx,
                );
            }
        };

        match x {
            TransmitType::InterComm(event) => match event {
                Some(val) => {
                    match val {
                        TxRxComm::Text(d) => tx.feed(Message::Text(d)).await,
                        // the implementation is here
                        // is it used? no
                        TxRxComm::Binary(d) => tx.feed(Message::Binary(d)).await,
                    };
                }
                None => break None,
            },
            TransmitType::Exit(reason) => break reason,
            TransmitType::Redis(Some(msg)) => {
                let uid = if let Some(uid) = uid_conn_map.get(&conn_id) {
                    *(uid.value())
                } else {
                    continue;
                };
                let bigdecimal_uid = u128_to_bigdecimal!(uid);

                match msg {
                    Some(msg) => {
                        match msg.get_channel::<String>() {
                            Ok(c) => {
                                let mut names = c.split('_');

                                // root event format: {type}_{event specific data}
                                match names.next() {
                                    Some("channel") => {
                                        if let (Some(Ok(channel_id)), Some(Ok(guild_id))) = (
                                            names.next().map(|x| x.parse::<u128>()),
                                            names.next().map(|x| x.parse::<u128>()),
                                        ) {
                                            // FIXME: once implemented, do a query to check the user has permissions to view channel in here

                                            // all checks completed, fire event
                                            let outbound_message = match simd_json::serde::from_reader::<_, WsOutboundEvent>(msg.get_payload_bytes()) {
                                                Ok(msg) => msg,
                                                Err(e) => {
                                                    return (
                                                        Some(CloseFrame {
                                                            code: CloseCode::from(5005),
                                                            reason: format!("Internal JSON representation decoding failed: {}", e).into(),
                                                        }),
                                                        tx,
                                                    )
                                                }
                                            };

                                            match outbound_message {
                                                WsOutboundEvent::ChannelDelete { .. } => (),
                                                _ => {
                                                    match sqlx::query!("SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2", bigdecimal_uid, u128_to_bigdecimal!(guild_id)).fetch_optional(db).await {
                                                        Ok(val) => {
                                                            match val {
                                                                Some(_) => (),
                                                                None => continue,
                                                            }
                                                        },
                                                        Err(e) => {
                                                            return (
                                                                Some(CloseFrame {
                                                                    code: CloseCode::from(5000),
                                                                    reason: format!(
                                                                        "Internal database error: {}",
                                                                        e
                                                                    )
                                                                        .into(),
                                                                }),
                                                                tx,
                                                            )
                                                        }
                                                    }
                                                }
                                            }
                                            let outbound_message =
                                                match simd_json::to_string(&outbound_message) {
                                                    Ok(msg) => msg,
                                                    Err(e) => {
                                                        return (
                                                            Some(CloseFrame {
                                                                code: CloseCode::from(5001),
                                                                reason: format!(
                                                                    "JSON serialization error: {}",
                                                                    e
                                                                )
                                                                .into(),
                                                            }),
                                                            tx,
                                                        )
                                                    }
                                                };
                                            tx.feed(Message::Text(outbound_message)).await;
                                        }
                                    }
                                    Some("message") => {
                                        // message event format: message_{channel ID}_{guild ID}
                                        if let (Some(Ok(channel_id)), Some(Ok(guild_id))) = (
                                            names.next().map(|x| x.parse::<u128>()),
                                            names.next().map(|x| x.parse::<u128>()),
                                        ) {
                                            // FIXME: once implemented, do a query to check the user has permissions to read messages in here
                                            match sqlx::query!("SELECT guild_id FROM members WHERE user_id = $1 AND guild_id = $2", bigdecimal_uid, u128_to_bigdecimal!(guild_id)).fetch_optional(db).await {
                                                Ok(val) => {
                                                    match val {
                                                        Some(_) => (),
                                                        None => continue,
                                                    }
                                                }
                                                Err(e) => {
                                                    return (
                                                        Some(CloseFrame {
                                                            code: CloseCode::from(5000),
                                                            reason: format!("Internal database error: {}", e).into(),
                                                        }),
                                                        tx,
                                                    )
                                                }
                                            }

                                            // all checks completed, fire event
                                            let outbound_message = match simd_json::serde::from_reader::<_, WsOutboundEvent>(msg.get_payload_bytes()) {
                                                Ok(msg) => msg,
                                                Err(e) => {
                                                    return (
                                                        Some(CloseFrame {
                                                            code: CloseCode::from(5005),
                                                            reason: format!("Internal JSON representation decoding failed: {}", e).into(),
                                                        }),
                                                        tx,
                                                    )
                                                }
                                            };
                                            let outbound_message =
                                                match simd_json::to_string(&outbound_message) {
                                                    Ok(msg) => msg,
                                                    Err(e) => {
                                                        return (
                                                            Some(CloseFrame {
                                                                code: CloseCode::from(5001),
                                                                reason: format!(
                                                                    "JSON serialization error: {}",
                                                                    e
                                                                )
                                                                .into(),
                                                            }),
                                                            tx,
                                                        )
                                                    }
                                                };
                                            tx.feed(Message::Text(outbound_message)).await;
                                        }
                                    }
                                    Some("guild") => {
                                        if let Some(Ok(guild_id)) =
                                            names.next().map(|x| x.parse::<u128>())
                                        {
                                            // FIXME: once implemented, do a query to check the user has permissions to read messages in here

                                            // all checks completed, fire event
                                            let outbound_message = match simd_json::serde::from_reader::<_, WsOutboundEvent>(msg.get_payload_bytes()) {
                                                Ok(msg) => msg,
                                                Err(e) => {
                                                    return (
                                                        Some(CloseFrame {
                                                            code: CloseCode::from(5005),
                                                            reason: format!("Internal JSON representation decoding failed: {}", e).into(),
                                                        }),
                                                        tx,
                                                    )
                                                }
                                            };

                                            match outbound_message {
                                                WsOutboundEvent::GuildDelete { .. } => (),
                                                _ => {
                                                    match sqlx::query!("SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2", bigdecimal_uid, u128_to_bigdecimal!(guild_id)).fetch_optional(db).await {
                                                        Ok(val) => {
                                                            match val {
                                                                Some(_) => (),
                                                                None => continue,
                                                            }
                                                        },
                                                        Err(e) => {
                                                            return (
                                                                Some(CloseFrame {
                                                                    code: CloseCode::from(5000),
                                                                    reason: format!("Internal database error: {}", e).into(),
                                                                }),
                                                                tx,
                                                            )
                                                        }
                                                    }
                                                }
                                            }

                                            let outbound_message =
                                                match simd_json::to_string(&outbound_message) {
                                                    Ok(msg) => msg,
                                                    Err(e) => {
                                                        return (
                                                            Some(CloseFrame {
                                                                code: CloseCode::from(5001),
                                                                reason: format!(
                                                                    "JSON serialization error: {}",
                                                                    e
                                                                )
                                                                .into(),
                                                            }),
                                                            tx,
                                                        )
                                                    }
                                                };

                                            tx.feed(Message::Text(outbound_message)).await;
                                        }
                                    }
                                    Some("member") => {
                                        if let Some(Ok(guild_id)) =
                                            names.next().map(|x| x.parse::<u128>())
                                        {
                                            // FIXME: once implemented, do a query to check the user has permissions to read messages in here

                                            // all checks completed, fire event
                                            let outbound_message = match simd_json::serde::from_reader::<_, WsOutboundEvent>(msg.get_payload_bytes()) {
                                                Ok(msg) => msg,
                                                Err(e) => {
                                                    return (
                                                        Some(CloseFrame {
                                                            code: CloseCode::from(5005),
                                                            reason: format!("Internal JSON representation decoding failed: {}", e).into(),
                                                        }),
                                                        tx,
                                                    )
                                                }
                                            };

                                            match outbound_message {
                                                WsOutboundEvent::MemberDelete { .. } => (),
                                                _ => {
                                                    match sqlx::query!("SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2", bigdecimal_uid, u128_to_bigdecimal!(guild_id)).fetch_optional(db).await {
                                                        Ok(val) => {
                                                            match val {
                                                                Some(_) => (),
                                                                None => continue,
                                                            }
                                                        },
                                                        Err(e) => {
                                                            return (
                                                                Some(CloseFrame {
                                                                    code: CloseCode::from(5000),
                                                                    reason: format!("Internal database error: {}", e).into(),
                                                                }),
                                                                tx,
                                                            )
                                                        }
                                                    }
                                                }
                                            }

                                            let outbound_message =
                                                match simd_json::to_string(&outbound_message) {
                                                    Ok(msg) => msg,
                                                    Err(e) => {
                                                        return (
                                                            Some(CloseFrame {
                                                                code: CloseCode::from(5001),
                                                                reason: format!(
                                                                    "JSON serialization error: {}",
                                                                    e
                                                                )
                                                                .into(),
                                                            }),
                                                            tx,
                                                        )
                                                    }
                                                };

                                            tx.feed(Message::Text(outbound_message)).await;
                                        }
                                    }
                                    Some("invite") => {
                                        if let Some(Ok(guild_id)) =
                                            names.next().map(|x| x.parse::<u128>())
                                        {
                                            // FIXME: once implemented, do a query to check the user has permissions to read messages in here

                                            // all checks completed, fire event
                                            let outbound_message = match simd_json::serde::from_reader::<_, WsOutboundEvent>(msg.get_payload_bytes()) {
                                                Ok(msg) => msg,
                                                Err(e) => {
                                                    return (
                                                        Some(CloseFrame {
                                                            code: CloseCode::from(5005),
                                                            reason: format!("Internal JSON representation decoding failed: {}", e).into(),
                                                        }),
                                                        tx,
                                                    )
                                                }
                                            };

                                            match outbound_message {
                                                WsOutboundEvent::MemberDelete { .. } => (),
                                                _ => {
                                                    match sqlx::query!("SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2", bigdecimal_uid, u128_to_bigdecimal!(guild_id)).fetch_optional(db).await {
                                                        Ok(val) => {
                                                            match val {
                                                                Some(_) => (),
                                                                None => continue,
                                                            }
                                                        },
                                                        Err(e) => {
                                                            return (
                                                                Some(CloseFrame {
                                                                    code: CloseCode::from(5000),
                                                                    reason: format!("Internal database error: {}", e).into(),
                                                                }),
                                                                tx,
                                                            )
                                                        }
                                                    }
                                                }
                                            }

                                            let outbound_message =
                                                match simd_json::to_string(&outbound_message) {
                                                    Ok(msg) => msg,
                                                    Err(e) => {
                                                        return (
                                                            Some(CloseFrame {
                                                                code: CloseCode::from(5001),
                                                                reason: format!(
                                                                    "JSON serialization error: {}",
                                                                    e
                                                                )
                                                                .into(),
                                                            }),
                                                            tx,
                                                        )
                                                    }
                                                };

                                            tx.feed(Message::Text(outbound_message)).await;
                                        }
                                    }
                                    Some(_) | None => continue,
                                }
                            }
                            Err(_) => continue,
                        };
                    }
                    None => {}
                }
            }
            TransmitType::Redis(None) => break None,
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
                let (redis_tx, redis_rx_2) = tokio::sync::mpsc::channel(250);
                redis_rx = Some(redis_rx_2);
                match crate::SUB_TO_ME.get() {
                    Some(s) => {
                        let user_id = *(map_val.value());
                        let mut s = s.clone();
                        s.start_send((format!("*{}*", user_id), redis_tx.clone()));
                        let resp = sqlx::query!(
                            "SELECT guild_id FROM members WHERE user_id = $1",
                            u128_to_bigdecimal!(user_id)
                        )
                        .fetch_all(db)
                        .await;
                        match resp {
                            Ok(resp) => {
                                for guild in resp.iter().filter_map(|x| {
                                    x.guild_id
                                        .with_scale(0)
                                        .into_bigint_and_exponent()
                                        .0
                                        .to_u128()
                                }) {
                                    s.start_send((format!("*{}*", guild), redis_tx.clone()));
                                }
                            }
                            Err(e) => {
                                return (
                                    Some(CloseFrame {
                                        code: CloseCode::from(5000),
                                        reason: format!("Internal database error: {}", e).into(),
                                    }),
                                    tx,
                                )
                            }
                        }
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

    (ret, tx)
}
