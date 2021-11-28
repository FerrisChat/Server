use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{
    Message, User, UserFlags,
};
use serde::Serialize;

/// GET `/api/v0/guilds/{guild_id}/channels/{channel_id}/messages/{message_id}`
pub async fn get_message(Path((channel_id, message_id)): Path<(u128, u128)>, auth: crate::Authorization) -> Result<crate::Json<Message>, WebServerError<impl Serialize>> {
    let bigint_message_id = u128_to_bigdecimal!(message_id);
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let db = get_db_or_fail!();

    let m = sqlx::query!(
        "SELECT m.*, a.pronouns AS author_pronouns, a.avatar AS author_avatar, a.name AS author_name, a.flags AS author_flags, a.discriminator AS author_discriminator FROM messages m CROSS JOIN LATERAL (SELECT * FROM users WHERE id = m.author_id) AS a WHERE m.id = $1 AND m.channel_id = $2",
        bigint_message_id,
        bigint_channel_id,
    )
        .fetch_optional(db)
        .await
        .map_err(|e| WebServerError::Database(e));

    let author_id = bigdecimal_to_u128!(m.author_id);
    let msg_obj = Message {
        id: message_id,
        content: m.content,
        channel_id,
        author_id: author_id.clone(),
        edited_at: m.edited_at,
        embeds: vec![],
        author: Some(User {
            id: author_id,
            name: m.author_name,
            avatar: m.avatar,
            guilds: None,
            flags: UserFlags::from_bits_truncate(m.author_flags),
            discriminator: m.author_discriminator,
            pronouns: m.pronouns,
        }),
        nonce: None,
    };

    Ok(crate::Json {
        obj: msg_obj,
        code: 200,
    })
}
