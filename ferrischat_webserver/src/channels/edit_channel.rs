use crate::ws::fire_event;
use crate::{Json, WebServerError};
use axum::extract::Path;
use ferrischat_common::request_json::ChannelUpdateJson;
use ferrischat_common::types::{Channel, ErrorJson};
use ferrischat_common::ws::WsOutboundEvent;
use serde::Serialize;

/// PATCH `/api/v0/channels/{channel_id}`
pub async fn edit_channel(
    Path(channel_id): Path<u128>,
    channel_info: axum::extract::Json<ChannelUpdateJson>,
    _: crate::Authorization,
) -> Result<Json<Channel>, WebServerError> {
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let db = get_db_or_fail!();
    let ChannelUpdateJson { name } = channel_info.0;

    let old_channel_obj = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(db)
        .await?
        .map(|c| Channel {
            id: channel_id,
            name: c.name,
            guild_id: bigdecimal_to_u128!(c.guild_id),
        })
        .ok_or_else(|| {
            (
                404,
                ErrorJson::new_404(
                    format!("Unknown channel with ID {}", channel_id),
                ),
            )
        })?;

    let new_channel_resp = sqlx::query!(
        "UPDATE channels SET name = $1 WHERE id= $2 RETURNING *",
        name,
        bigint_channel_id
    )
    .fetch_optional(db)
    .await?;
    let new_channel_obj = Channel {
        id: channel_id,
        name: new_channel_resp.name,
        guild_id: bigdecimal_to_u128!(new_channel_resp.guild_id),
    };

    let event = WsOutboundEvent::ChannelUpdate {
        old: old_channel_obj,
        new: new_channel_obj.clone(),
    };

    fire_event(
        format!("channel_{}_{}", channel_id, new_channel_obj.guild_id),
        &event,
    )
    .await?;

    Ok(Json {
        obj: new_channel_obj,
        code: 200,
    })
}
