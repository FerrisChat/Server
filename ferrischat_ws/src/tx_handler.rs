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

use ferrischat_redis::redis::Msg;

use crate::events::*;
use crate::inter_communication::TxRxComm;
use crate::USERID_CONNECTION_MAP;

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

    let ret = loop {
        let x = match redis_rx {
            Some(ref mut rx) => tokio::select! {
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
                    let _ = match val {
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
                        return (Some(e), tx);
                    }
                }
            }
            TransmitType::Redis(None) => break None,
        }
        let _ = tx.flush().await;

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
                        if s.start_send((format!("*{}*", user_id), redis_tx.clone()))
                            .is_err()
                        {
                            return (
                                Some(CloseFrame {
                                    code: CloseCode::from(5005),
                                    reason: "Redis connection pool hung up connection".into(),
                                }),
                                tx,
                            );
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
                                    if s.start_send((format!("*{}*", guild), redis_tx.clone()))
                                        .is_err()
                                    {
                                        return (
                                            Some(CloseFrame {
                                                code: CloseCode::from(5005),
                                                reason: "Redis connection pool hung up connection"
                                                    .into(),
                                            }),
                                            tx,
                                        );
                                    }
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
        let _ = tx.flush().await;
    };

    (ret, tx)
}
