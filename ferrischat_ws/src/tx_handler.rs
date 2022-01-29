use crate::event::RedisMessage;
#[allow(clippy::wildcard_imports)]
use crate::events::*;
use crate::USERID_CONNECTION_MAP;
use ferrischat_common::ws::WsOutboundEvent;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use num_traits::ToPrimitive;
use tokio::net::UnixStream;
use tokio::sync::mpsc::Receiver;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

pub async fn tx_handler(
    mut tx: SplitSink<WebSocketStream<UnixStream>, Message>,
    mut closer_rx: futures::channel::oneshot::Receiver<Option<CloseFrame<'_>>>,
    mut inter_rx: Receiver<WsOutboundEvent>,
    conn_id: Uuid,
) -> (
    Option<CloseFrame<'_>>,
    SplitSink<WebSocketStream<UnixStream>, Message>,
) {
    enum TransmitType<'t> {
        InterComm(Box<Option<WsOutboundEvent>>),
        Exit(Option<CloseFrame<'t>>),
        Redis(Option<RedisMessage>),
    }

    let mut redis_rx: Option<Receiver<Option<RedisMessage>>> = None;

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
        let x = if let Some(ref mut rx) = redis_rx {
            tokio::select! {
                item = &mut closer_rx => TransmitType::Exit(item.ok().flatten()),
                item = inter_rx.recv() => TransmitType::InterComm(box item),
                item = rx.recv() => TransmitType::Redis(item.flatten()),
            }
        } else {
            tokio::select! {
                item = &mut closer_rx => TransmitType::Exit(item.ok().flatten()),
                item = inter_rx.recv() => TransmitType::InterComm(box item),
            }
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
                    if let Err(e) = tx.feed(Message::Text(payload)).await {
                        error!("failed to send message: {:?}", e);
                    }
                }
                None => {
                    break Some(CloseFrame {
                        code: CloseCode::from(1000),
                        reason: "normal close".into(),
                    })
                }
            },
            TransmitType::Exit(reason) => break reason,
            TransmitType::Redis(Some(mut msg)) => {
                let uid = if let Some(uid) = uid_conn_map.get(&conn_id) {
                    *(uid.value())
                } else {
                    continue;
                };

                let n = msg.channel;
                let outbound_message =
                    match simd_json::serde::from_str::<WsOutboundEvent>(&mut msg.message) {
                        Ok(msg) => msg,
                        Err(e) => {
                            break Some(CloseFrame {
                                code: CloseCode::from(5005),
                                reason: format!(
                                    "Internal JSON representation decoding failed: {}",
                                    e
                                )
                                .into(),
                            })
                        }
                    };
                let mut names = n.split('_');
                let item_name = if let Some(n) = names.next() {
                    n
                } else {
                    warn!(obj = %n, "event name was missing a type");
                    continue;
                };
                let obj_id = match names.next().map(str::parse::<u128>) {
                    Some(Ok(id)) => id,
                    Some(Err(e)) => {
                        warn!(obj = %n, "failed to parse object ID as u128: {}", e);
                        continue;
                    }
                    None => {
                        warn!(obj = %n, "object was missing an ID");
                        continue;
                    }
                };
                let ret = match item_name {
                    "channel" => {
                        ChannelEvent::handle_event(db, &outbound_message, uid, obj_id).await
                    }
                    "message" => {
                        MessageEvent::handle_event(db, &outbound_message, uid, obj_id).await
                    }
                    // note we don't handle the special `gc` case here
                    "guild" => GuildEvent::handle_event(db, &outbound_message, uid, obj_id).await,
                    "member" => MemberEvent::handle_event(db, &outbound_message, uid, obj_id).await,
                    "invite" => InviteEvent::handle_event(db, &outbound_message, uid, obj_id).await,
                    t => {
                        warn!("unknown event type {}", t);
                        continue;
                    }
                };
                match ret {
                    Ok(true) => {
                        let payload = msg.message;
                        if let Err(e) = tx.feed(Message::Text(payload)).await {
                            warn!("Error while sending message to WebSocket client: {:?}", e);
                        }
                    }
                    Ok(false) => {}
                    Err(e) => {
                        break Some(e.into());
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
        if let Err(e) = tx.flush().await {
            warn!("error while flushing client websocket: {:?}", e);
        }
    };

    (ret, tx)
}
