use crate::error_handling::WsEventHandlerError;
use dashmap::DashMap;
use ferrischat_auth::{split_token, verify_token};
use ferrischat_common::types::UserFlags;
use ferrischat_common::ws::{Intents, WsOutboundEvent};
use num_traits::ToPrimitive;
use sqlx::{Pool, Postgres};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use uuid::Uuid;

pub async fn handle_identify_rx<'a>(
    token: String,
    _intents: Intents,
    inter_tx: &Sender<WsOutboundEvent>,
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

    let (id, secret) = split_token(token.as_str())?;
    verify_token(id, secret).await?;
    let bigdecimal_user_id = u128_to_bigdecimal!(id);

    let res = sqlx::query!("SELECT * FROM users WHERE id = $1", bigdecimal_user_id)
        .fetch_one(db)
        .await;

    let is_bot = false;
    match res {
        Ok(ref u) => {
            if UserFlags::from_bits_truncate(u.flags).contains(UserFlags::BOT_ACCOUNT) {
                let _is_bot = true;
            }
        }
        Err(_) => (),
    }

    let guilds = {
        let d = sqlx::query!(
            r#"SELECT id AS "id!", owner_id AS "owner_id!", name AS "name!", icon, flags AS "flags!" FROM guilds INNER JOIN members m on guilds.id = m.guild_id WHERE m.user_id = $1"#,
            bigdecimal_user_id
        )
                .fetch_all(db)
                .await?;

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
            let icon = x.icon;
            let flags = x.flags;

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
                                "SELECT m.*, u.avatar AS avatar, u.name AS name, u.discriminator AS discriminator, u.flags AS flags, u.pronouns AS pronouns FROM members m \
                                CROSS JOIN LATERAL (SELECT * FROM users u WHERE id = m.user_id) AS u WHERE guild_id = $1",
                                x.id.clone())
                    .fetch_all(db)
                    .await?;

                Some(
                    resp.iter()
                        .filter_map(|x| {
                            let user_id = x
                                .user_id
                                .with_scale(0)
                                .into_bigint_and_exponent()
                                .0
                                .to_u128()?;
                            let is_bot_m = false;
                            if UserFlags::from_bits_truncate(x.flags)
                                .contains(UserFlags::BOT_ACCOUNT)
                            {
                                let _is_bot_m = true;
                            }
                            Some(ferrischat_common::types::Member {
                                user_id: Some(user_id),
                                user: Some(ferrischat_common::types::User {
                                    id: user_id,
                                    name: x.name.clone(),
                                    avatar: x.avatar.clone(),
                                    guilds: None,
                                    flags: ferrischat_common::types::UserFlags::from_bits_truncate(
                                        x.flags,
                                    ),
                                    discriminator: x.discriminator,
                                    pronouns: x
                                        .pronouns
                                        .and_then(ferrischat_common::types::Pronouns::from_i16),
                                    is_bot: is_bot_m,
                                }),
                                guild_id: Some(id),
                                guild: None,
                            })
                        })
                        .collect(),
                )
            };

            let channels = {
                let resp = sqlx::query!("SELECT * FROM channels WHERE guild_id = $1", x.id.clone())
                    .fetch_all(db)
                    .await?;

                Some(
                    resp.iter()
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
                )
            };

            guilds.push(ferrischat_common::types::Guild {
                id,
                owner_id,
                name: x.name.clone(),
                channels,
                flags: ferrischat_common::types::GuildFlags::from_bits_truncate(flags),
                members,
                roles: None,
                icon,
            });
        }
        Some(guilds)
    };

    let user = match res {
        Ok(u) => ferrischat_common::types::User {
            id,
            name: u.name,
            avatar: u.avatar,
            guilds,
            flags: ferrischat_common::types::UserFlags::from_bits_truncate(u.flags),
            discriminator: u.discriminator,
            pronouns: u
                .pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
            is_bot,
        },
        Err(e) => {
            return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                code: CloseCode::from(5000),
                reason: format!("Internal database error: {}", e).into(),
            }))
        }
    };

    inter_tx
        .send(WsOutboundEvent::IdentifyAccepted { user })
        .await
        .as_ref()?;

    uid_conn_map.insert(conn_id, id);

    Ok(())
}
