use crate::ws::{fire_event, WsEventError};
use axum::extract::Path;

use ferrischat_common::ws::WsOutboundEvent;

use crate::{Json, WebServerError};
use actix_web::{HttpRequest, HttpResponse, Responder};
use axum::Json as JsonInput;
use ferrischat_common::request_json::ChannelCreateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, ModelType};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;
use serde::Serialize;

/// POST `/api/v0/guilds/{guild_id/channels`
pub async fn create_channel(
    _: crate::Authorization,
    channel_info: JsonInput<ChannelCreateJson>,
    Path(channel_id): Path<u128>,
) -> Result<Json<Channel>, WebServerError<impl Serialize>> {
    let db = get_db_or_fail!();

    let ChannelCreateJson { name } = channel_info.0;

    let node_id = get_node_id!();
    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, node_id);
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    sqlx::query!(
        "INSERT INTO channels VALUES ($1, $2, $3)",
        bigint_channel_id,
        name,
        bigint_guild_id
    )
    .execute(db)
    .await
    .map_err(|e| WebServerError::Database(e))?;

    let channel_obj = Channel {
        id: channel_id,
        name,
        guild_id,
    };

    let event = WsOutboundEvent::ChannelCreate {
        channel: channel_obj.clone(),
    };

    fire_event(format!("channel_{}_{}", guild_id, channel_id), &event).await?;

    Ok(crate::Json {
        obj: channel_obj,
        code: 201,
    })
}
