use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::{Json, Path};
use ferrischat_common::request_json::MessageCreateJson;
use ferrischat_common::types::{Channel, ErrorJson, Message, ModelType, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST `/api/v0/channels/{channel_id}/messages`
pub async fn create_message(
    auth: crate::Authorization,
    json: Json<MessageCreateJson>,
    Path(channel_id): Path<u128>,
) -> Result<crate::Json<Message>, WebServerError> {
    let MessageCreateJson { content, nonce } = json.0;

    if content.len() > 10240 {
        return Err(ErrorJson::new_400(
            "message content size must be fewer than 10,240 bytes".to_string(),
        )
        .into());
    }

    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let node_id = {
        ferrischat_redis::NODE_ID
            .get()
            .copied()
            .ok_or(WebServerError::MissingNodeId)?
    };
    let message_id = generate_snowflake::<0>(ModelType::Message as u8, node_id);
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let author_id = auth.0;
    let bigint_author_id = u128_to_bigdecimal!(author_id);

    let db = get_db_or_fail!();

    let channel = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404("channel not found".to_string()))?;

    let channel_obj = Channel {
        id: channel_id,
        name: channel.name,
        guild_id: bigdecimal_to_u128!(channel.guild_id),
    };

    sqlx::query!(
        "INSERT INTO messages VALUES ($1, $2, $3, $4)",
        bigint_message_id,
        content,
        bigint_channel_id,
        bigint_author_id
    )
    .execute(db)
    .await?;

    let r = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_author_id)
        .fetch_one(db)
        .await?;
    let author = User {
        id: bigdecimal_to_u128!(r.id),
        name: r.name,
        avatar: r.avatar,
        guilds: None,
        flags: UserFlags::from_bits_truncate(r.flags),
        discriminator: r.discriminator,
        pronouns: r
            .pronouns
            .and_then(ferrischat_common::types::Pronouns::from_i16),
    };

    let msg_obj = Message {
        id: message_id,
        content: Some(content),
        channel: Some(channel_obj),
        channel_id,
        author_id,
        author: Some(author),
        edited_at: None,
        embeds: vec![],
        nonce,
    };

    let event = WsOutboundEvent::MessageCreate {
        message: msg_obj.clone(),
    };

    fire_event(
        format!("message_{}_{}", channel_id, bigdecimal_to_u128!(channel.guild_id)),
        &event,
    )
    .await?;

    Ok(crate::Json {
        obj: msg_obj,
        code: 201,
    })
}
