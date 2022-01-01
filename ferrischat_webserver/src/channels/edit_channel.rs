use crate::ws::fire_event;
use crate::{Json, WebServerError};
use axum::extract::Path;
use ferrischat_common::request_json::ChannelUpdateJson;
use ferrischat_common::types::{Channel, ErrorJson};
use ferrischat_common::ws::WsOutboundEvent;

/// PATCH `/v0/channels/{channel_id}`
pub async fn edit_channel(
    Path(channel_id): Path<u128>,
    channel_info: axum::extract::Json<ChannelUpdateJson>,
    _: crate::Authorization,
) -> Result<Json<Channel>, WebServerError> {
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let db = get_db_or_fail!();
    let ChannelUpdateJson { name } = channel_info.0;

    let c = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown channel with ID {}", channel_id)))?;
    let old = Channel {
        id: channel_id,
        name: c.name,
        guild_id: bigdecimal_to_u128!(c.guild_id),
    };

    if let Some(name) = name {
        if name.contains(char::is_whitespace) {
            return Err(ErrorJson::new_400(
                "A channel name may not contain a whitespace!".to_string(),
            )
            .into());
        }
        sqlx::query!(
            "UPDATE channels SET name = $1 WHERE id = $2",
            name,
            bigint_channel_id
        )
        .execute(db)
        .await?;
    }

    let channel = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown channel with ID {}", channel_id)))?;
    let new = Channel {
        id: channel_id,
        name: channel.name,
        guild_id: bigdecimal_to_u128!(channel.guild_id),
    };

    let event = WsOutboundEvent::ChannelUpdate {
        old,
        new: new.clone(),
    };

    fire_event(&event).await?;

    Ok(Json {
        obj: new,
        code: 200,
    })
}
