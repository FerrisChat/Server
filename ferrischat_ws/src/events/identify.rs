use crate::error_handling::WsEventHandlerError;
use crate::TxRxComm;
use dashmap::DashMap;
use ferrischat_auth::{split_token, verify_token};
use ferrischat_common::ws::WsOutboundEvent;
use num_traits::ToPrimitive;
use sqlx::{Pool, Postgres};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use uuid::Uuid;

pub async fn handle_identify_rx<'a>(
    token: String,
    _intents: u64,
    inter_tx: &Sender<TxRxComm>,
    uid_conn_map: &DashMap<Uuid, u128>,
    identify_received: &AtomicBool,
    db: &Pool<Postgres>,
    conn_id: Uuid,
) -> Result<(), WsEventHandlerError<'a>> {
    if identify_received.swap(true, Ordering::Relaxed) {
        return Err(WsEventHandlerError::CloseFrame(CloseFrame {
            code: CloseCode::from(2002),
            reason: "Too many IDENTIFY payloads sent".into(),
        }));
    }

    let (id, secret) = match split_token(token) {
        Ok((id, secret)) => (id, secret),
        Err(_) => {
            return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                code: CloseCode::from(2003),
                reason: "Token invalid".into(),
            }))
        }
    };
    if let Err(_) = verify_token(id, secret).await {
        return Err(WsEventHandlerError::CloseFrame(CloseFrame {
            code: CloseCode::from(2003),
            reason: "Token invalid".into(),
        }));
    }
    let bigdecimal_user_id = u128_to_bigdecimal!(id);

    let res = sqlx::query!("SELECT * FROM users WHERE id = $1", bigdecimal_user_id)
        .fetch_one(db)
        .await;

    let guilds = {
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
                    let id = match x
                        .id
                        .clone()
                        .with_scale(0)
                        .into_bigint_and_exponent()
                        .0
                        .to_u128()
                    {
                        Some(id) => id,
                        None => {
                            return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                                code: CloseCode::from(5006),
                                reason: "Failed to parse ID as u128".into(),
                            }))
                        }
                    };

                    let owner_id = match x
                        .owner_id
                        .with_scale(0)
                        .into_bigint_and_exponent()
                        .0
                        .to_u128()
                    {
                        Some(id) => id,
                        None => {
                            return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                                code: CloseCode::from(5006),
                                reason: "Failed to parse ID as u128".into(),
                            }))
                        }
                    };

                    let members = {
                        let resp = sqlx::query!(
                                "SELECT m.*, u.name AS name, u.discriminator AS discriminator, u.flags AS flags FROM members m \
                                CROSS JOIN LATERAL (SELECT * FROM users u WHERE id = m.user_id) AS u WHERE guild_id = $1",
                                x.id.clone())
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
                                        guild_id: Some(id),
                                        guild: None,
                                    })
                                })
                                .collect(),
                            Err(e) => {
                                return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                                    code: CloseCode::from(5000),
                                    reason: format!("Internal database error: {}", e).into(),
                                }))
                            }
                        })
                    };

                    let channels = {
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
                                        id: x
                                            .id
                                            .clone()
                                            .with_scale(0)
                                            .into_bigint_and_exponent()
                                            .0
                                            .to_u128()?,
                                        name: x.name.clone(),
                                        guild_id: id,
                                    })
                                })
                                .collect(),
                            Err(e) => {
                                return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                                    code: CloseCode::from(5000),
                                    reason: format!("Internal database error: {}", e).into(),
                                }))
                            }
                        })
                    };

                    guilds.push(ferrischat_common::types::Guild {
                        id,
                        owner_id,
                        name: x.name.clone(),
                        channels,
                        flags: ferrischat_common::types::GuildFlags::empty(),
                        members,
                        roles: None,
                    });
                }
                Some(guilds)
            }
            Err(e) => {
                return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                    code: CloseCode::from(5000),
                    reason: format!("Internal database error: {}", e).into(),
                }))
            }
        }
    };

    let user = match res {
        Ok(u) => ferrischat_common::types::User {
            id,
            name: u.name,
            avatar: None,
            guilds,
            flags: ferrischat_common::types::UserFlags::from_bits_truncate(u.flags),
            discriminator: u.discriminator,
        },
        Err(e) => {
            return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                code: CloseCode::from(5000),
                reason: format!("Internal database error: {}", e).into(),
            }))
        }
    };

    let payload = match simd_json::serde::to_string(&WsOutboundEvent::IdentifyAccepted { user }) {
        Ok(v) => v,
        Err(e) => {
            return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                code: CloseCode::from(5001),
                reason: format!("JSON serialization error: {}", e).into(),
            }))
        }
    };
    if let Err(_) = inter_tx.send(TxRxComm::Text(payload)).await {
        return Err(WsEventHandlerError::Sender);
    };

    uid_conn_map.insert(conn_id, id);

    Ok(())
}
