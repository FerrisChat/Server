use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Channel, ErrorJson};
use ferrischat_common::ws::WsOutboundEvent;

/// DELETE `/v0/channels/{channel_id}`
pub async fn delete_channel(
    Path(channel_id): Path<u128>,
    _: crate::Authorization,
) -> Result<http::StatusCode, WebServerError> {
    let db = get_db_or_fail!();
    let bigdecimal_channel_id = u128_to_bigdecimal!(channel_id);

    let channel = sqlx::query!(
        "DELETE FROM channels WHERE id = $1 RETURNING *",
        bigdecimal_channel_id,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ErrorJson::new_404(format!("Unknown channel with ID {}", channel_id)))?;
    let channel = Channel {
        id: bigdecimal_to_u128!(channel.id),
        guild_id: bigdecimal_to_u128!(channel.guild_id),
        name: channel.name,
    };

    let event = WsOutboundEvent::ChannelDelete { channel };

    fire_event(&event).await?;

    Ok(http::StatusCode::NO_CONTENT)
}
