use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use axum::Json;
use ferrischat_common::request_json::MessageUpdateJson;
use ferrischat_common::types::{Channel, ErrorJson, Message, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;

pub async fn edit_message(
    Path((channel_id, message_id)): Path<(u128, u128)>,
    Json(MessageUpdateJson { content }): Json<MessageUpdateJson>,
    crate::Authorization(user_id, _): crate::Authorization,
) -> Result<crate::Json<Message>, WebServerError> {
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let db = get_db_or_fail!();

    if let Some(ref content) = content {
        if content.len() > 10240 {
            return Err(ErrorJson::new_400(
                "message content size must be fewer than 10,240 bytes".to_string(),
            )
            .into());
        }
    }

    let channel = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404("channel not found".to_string()))?;

    let channel_obj = Channel {
        id: channel_id,
        name: channel.name,
        guild_id: bigdecimal_to_u128!(channel.guild_id),
    };

    let resp = sqlx::query!(
        "SELECT m.*, a.avatar AS avatar, a.name AS author_name, a.flags AS author_flags, a.discriminator AS author_discriminator, a.pronouns AS author_pronouns FROM messages m CROSS JOIN LATERAL (SELECT * FROM users WHERE id = m.author_id) AS a WHERE m.id = $1 AND m.channel_id = $2",
        bigint_message_id,
        bigint_channel_id,
    )
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown message with ID {}", message_id)),
        )?;

    let old_message_obj = {
        let author_id = bigdecimal_to_u128!(resp.author_id);
        if author_id != user_id {
            return Err(ErrorJson::new_403(
                "this user is not the author of the message".to_string(),
            )
            .into());
        }

        let author_id = bigdecimal_to_u128!(resp.author_id);

        Message {
            id: message_id,
            channel: channel_obj.clone(),
            channel_id,
            author_id,
            content: resp.content,
            edited_at: resp.edited_at,
            embeds: vec![],
            author: Some(User {
                id: author_id,
                name: resp.author_name,
                avatar: resp.avatar,
                guilds: None,
                flags: UserFlags::from_bits_truncate(resp.author_flags),
                discriminator: resp.author_discriminator,
                pronouns: resp
                    .author_pronouns
                    .and_then(ferrischat_common::types::Pronouns::from_i16),
                is_bot: {
                    UserFlags::from_bits_truncate(resp.author_flags)
                        .contains(UserFlags::BOT_ACCOUNT)
                },
            }),
            nonce: None,
        }
    };

    let message = sqlx::query!("UPDATE messages SET content = $1, edited_at = now()::timestamp without time zone WHERE channel_id = $2 AND id = $3 RETURNING *", content, bigint_channel_id, bigint_message_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(
            format!("Unknown message with ID {}", message_id)
        )
        )?;
    let new_msg_obj = Message {
        id: message_id,
        channel: channel_obj,
        channel_id,
        author_id: bigdecimal_to_u128!(message.author_id),
        content: message.content,
        edited_at: message.edited_at,
        embeds: vec![],
        author: old_message_obj.author.clone(),
        nonce: None,
    };

    let event = WsOutboundEvent::MessageUpdate {
        old: old_message_obj,
        new: new_msg_obj.clone(),
    };

    fire_event(&event).await?;
    Ok(crate::Json {
        obj: new_msg_obj,
        code: 200,
    })
}
