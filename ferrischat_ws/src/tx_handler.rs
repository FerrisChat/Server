use crate::events::*;
use crate::USERID_CONNECTION_MAP;
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
    mut inter_rx: tokio::sync::mpsc::Receiver<WsOutboundEvent>,
    conn_id: Uuid,
) -> (
    Option<CloseFrame<'_>>,
    SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>,
) {
    enum TransmitType<'t> {
        InterComm(Box<Option<WsOutboundEvent>>),
        Exit(Option<CloseFrame<'t>>),
        Redis(Option<Option<Msg>>),
    }

    let mut redis_rx: Option<tokio::sync::mpsc::Receiver<Option<Msg>>> = None;

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

    let ret = 'outer: loop {
        let x = match redis_rx {
            Some(ref mut rx) => tokio::select! {
                item = &mut closer_rx => TransmitType::Exit(item.ok().flatten()),
                item = inter_rx.recv() => TransmitType::InterComm(box item),
                item = rx.recv() => TransmitType::Redis(item),
            },
            None => tokio::select! {
                item = &mut closer_rx => TransmitType::Exit(item.ok().flatten()),
                item = inter_rx.recv() => TransmitType::InterComm(box item),
            },
        };

        match x {
            TransmitType::InterComm(event) => match event.into() {
                Some(val) => {
                    let payload = match simd_json::serde::to_string(&val) {
                        Ok(v) => v,
                        Err(e) => {
                            break Some(CloseFrame {
                                code: CloseCode::from(5001),
                                reason: format!("JSON serialization error: {}", e).into(),
                            });
                        }
                    };
                    let _ = tx.feed(Message::Text(payload));
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

                if let Some(msg) = msg {
                    let n = match msg.get_channel::<String>().ok() {
                        Some(n) => n,
                        None => continue,
                    };
                    let mut names = n.split('_');
                    let ret = match names.next() {
                        Some("channel") => {
                            if let (Some(Ok(channel_id)), Some(Ok(guild_id))) =
                                (names.next().map(str::parse), names.next().map(str::parse))
                            {
                                handle_channel_tx(
                                    &mut tx,
                                    db,
                                    msg,
                                    bigdecimal_uid,
                                    channel_id,
                                    guild_id,
                                )
                                .await
                            } else {
                                continue;
                            }
                        }
                        Some("message") => {
                            // message event format: message_{channel ID}_{guild ID}
                            if let (Some(Ok(channel_id)), Some(Ok(guild_id))) =
                                (names.next().map(str::parse), names.next().map(str::parse))
                            {
                                handle_message_tx(
                                    &mut tx,
                                    db,
                                    msg,
                                    bigdecimal_uid,
                                    channel_id,
                                    guild_id,
                                )
                                .await
                            } else {
                                continue;
                            }
                        }
                        Some("guild") => {
                            if let Some(Ok(guild_id)) = names.next().map(str::parse) {
                                handle_guild_tx(&mut tx, db, msg, bigdecimal_uid, guild_id).await
                            } else {
                                continue;
                            }
                        }
                        Some("member") => {
                            if let Some(Ok(guild_id)) = names.next().map(str::parse) {
                                handle_member_tx(&mut tx, db, msg, bigdecimal_uid, guild_id).await
                            } else {
                                continue;
                            }
                        }
                        Some("invite") => {
                            if let Some(Ok(guild_id)) = names.next().map(str::parse) {
                                handle_invite_tx(&mut tx, db, msg, bigdecimal_uid, guild_id).await
                            } else {
                                continue;
                            }
                        }
                        Some(_) | None => continue,
                    };
                    if let Err(e) = ret {
                        break Some(e);
                    }
                }
            }
            TransmitType::Redis(None) => {
                break Some(CloseFrame {
                    code: CloseCode::from(5007),
                    reason: "Redis failed to subscribe to channel".into(),
                })
            }
        }
        let _ = tx.flush().await;

        if redis_rx.is_none() {
            let uid_conn_map = match USERID_CONNECTION_MAP.get() {
                Some(m) => m,
                None => {
                    break Some(CloseFrame {
                        code: CloseCode::from(5004),
                        reason: "Connection map not found".into(),
                    });
                }
            };
            if let Some(map_val) = uid_conn_map.get(&conn_id) {
                let (redis_tx, redis_rx_2) = tokio::sync::mpsc::channel(250);
                redis_rx = Some(redis_rx_2);
                match crate::SUB_TO_ME.get() {
                    Some(s) => {
                        let user_id = *(map_val.value());
                        if s.send((format!("*{}*", user_id), redis_tx.clone()))
                            .await
                            .is_err()
                        {
                            break Some(CloseFrame {
                                code: CloseCode::from(5005),
                                reason: "Redis connection pool hung up connection".into(),
                            });
                        }
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
                                    if s.send((format!("*{}*", guild), redis_tx.clone()))
                                        .await
                                        .is_err()
                                    {
                                        break 'outer Some(CloseFrame {
                                            code: CloseCode::from(5006),
                                            reason: "Redis connection pool hung up connection"
                                                .into(),
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                break Some(CloseFrame {
                                    code: CloseCode::from(5000),
                                    reason: format!("Internal database error: {}", e).into(),
                                })
                            }
                        }
                    }
                    None => {
                        break Some(CloseFrame {
                            code: CloseCode::from(5002),
                            reason: "Redis pool not found".into(),
                        });
                    }
                };
            };
        }
        let _ = tx.flush().await;
    };

    (ret, tx)
}
