use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Message, ErrorJson, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;
use serde::Serialize;

/// DELETE `/api/v0/channels/{channel_id}/messages/{message_id}`
pub async fn delete_message(
    Path((channel_id, message_id)): Path<(u128, u128)>,
    _: crate::Authorization,
) -> Result<http::StatusCode, WebServerError> {
    let bigint_message_id = u128_to_bigdecimal!(message_id);
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let db = get_db_or_fail!();

    let guild_id: u128 = bigdecimal_to_u128!(sqlx::query!(
        "SELECT guild_id FROM channels WHERE id = $1",
        bigint_channel_id
    )
    .fetch_optional(db)
    .await?
    .map(|c| c.guild_id)
    .ok_or_else(|| {
        (
            404,
            ErrorJson::new_404(
                format!("Unknown channel with ID {}", channel_id),
            ),
        )
            .into()
    })?);

    let message = sqlx::query!(
        r#"
SELECT m.*,
       a.name AS author_name,
       a.flags AS author_flags,
       a.discriminator AS author_discriminator,
       a.pronouns AS author_pronouns
FROM messages m
    CROSS JOIN LATERAL (
        SELECT *
        FROM users
        WHERE id = m.author_id
        ) AS a 
WHERE m.id = $1
  AND m.channel_id = $2
  "#,
        bigint_message_id,
        bigint_channel_id,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        (
            404,
            ErrorJson::new_404(
                format!("Unknown message with ID {}", message_id),
            ),
        )
            .into()
    })?;

    let author_id = bigdecimal_to_u128!(message.author_id);

    let msg_obj = Message {
        id: message_id,
        channel_id,
        author_id,
        content: message.content,
        edited_at: message.edited_at,
        embeds: vec![],
        author: Some(User {
            id: author_id,
            name: message.author_name,
            avatar: None,
            guilds: None,
            flags: UserFlags::from_bits_truncate(message.author_flags),
            discriminator: message.author_discriminator,
            pronouns: message
                .author_pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
        }),
        nonce: None,
    };

    sqlx::query!(
        "DELETE FROM messages WHERE id = $1 AND channel_id = $2",
        bigint_message_id,
        bigint_channel_id
    )
    .execute(db)
    .await?;

    let event = WsOutboundEvent::MessageDelete {
        message: msg_obj.clone(),
    };

    fire_event(format!("message_{}_{}", channel_id, guild_id), &event).await?;
    Ok(http::StatusCode::NO_CONTENT)
}
