use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Channel, NotFoundJson};
use ferrischat_common::ws::WsOutboundEvent;
use serde::Serialize;

/// DELETE `/api/v0/channels/{channel_id}`
pub async fn delete_channel(
    Path(channel_id): Path<u128>,
    _: crate::Authorization,
) -> Result<http::StatusCode, WebServerError<impl Serialize>> {
    let db = get_db_or_fail!();
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let channel_obj = sqlx::query!(
        "DELETE FROM channels WHERE id = $1 RETURNING *",
        bigint_channel_id,
    )
    .fetch_optional(db)
    .await?
    .map(|channel| Channel {
        id: bigdecimal_to_u128!(channel.id),
        guild_id: bigdecimal_to_u128!(channel.guild_id),
        name: channel.name,
    })
    .ok_or_else(|| {
        (
            404,
            NotFoundJson {
                message: format!("Unknown channel with ID {}", channel_id),
            },
        )
    })?;

    let event = WsOutboundEvent::ChannelDelete {
        channel: channel_obj.clone(),
    };

    fire_event(
        format!("channel_{}_{}", channel_id, channel_obj.guild_id),
        &event,
    )
    .await?;

    Ok(http::StatusCode::NO_CONTENT)
}
