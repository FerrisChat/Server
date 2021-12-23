use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Channel, ErrorJson, Message, User, UserFlags};

/// GET `/v0/guilds/{guild_id}/channels/{channel_id}/messages/{message_id}`
pub async fn get_message(
    Path((channel_id, message_id)): Path<(u128, u128)>,
    crate::Authorization(_, is_bot): crate::Authorization,
) -> Result<crate::Json<Message>, WebServerError> {
    let db = get_db_or_fail!();
    let bigint_message_id = u128_to_bigdecimal!(message_id);
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let channel = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404("channel not found".to_string()))?;

    let channel_obj = Channel {
        id: channel_id,
        name: channel.name,
        guild_id: bigdecimal_to_u128!(channel.guild_id),
    };

    let m =
        sqlx::query!(
            "SELECT m.*, a.pronouns AS author_pronouns, a.avatar AS author_avatar, a.name AS author_name, a.flags AS author_flags, a.discriminator AS author_discriminator FROM messages m CROSS JOIN LATERAL (SELECT * FROM users WHERE id = m.author_id) AS a WHERE m.id = $1 AND m.channel_id = $2",
            bigint_message_id,
            bigint_channel_id
        )
            .fetch_optional(db)
            .await?
            .ok_or_else(|| {
                ErrorJson::new_404(
                    format!("Unknown message with ID {}", message_id),
                )
            })?;
    Ok(crate::Json {
        obj: Message {
            id: message_id,
            content: m.content,
            channel: channel_obj,
            channel_id,
            author_id: bigdecimal_to_u128!(m.author_id),
            edited_at: m.edited_at,
            embeds: vec![],
            author: Some(User {
                id: bigdecimal_to_u128!(m.author_id),
                name: m.author_name,
                avatar: m.author_avatar,
                guilds: None,
                flags: UserFlags::from_bits_truncate(m.author_flags),
                discriminator: m.author_discriminator,
                pronouns: m
                    .author_pronouns
                    .and_then(ferrischat_common::types::Pronouns::from_i16),
                is_bot,
            }),
            nonce: None,
        },
        code: 200,
    })
}
