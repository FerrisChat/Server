use crate::ws::fire_event;
use crate::{Json, WebServerError};
use axum::extract::Path;
use axum::Json as JsonInput;
use ferrischat_common::request_json::ChannelCreateJson;
use ferrischat_common::types::{Channel, ErrorJson, ModelType};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST `/v0/guilds/{guild_id/channels`
pub async fn create_channel(
    _: crate::Authorization,
    channel_info: JsonInput<ChannelCreateJson>,
    Path(guild_id): Path<u128>,
) -> Result<Json<Channel>, WebServerError> {
    let db = get_db_or_fail!();

    let ChannelCreateJson { name } = channel_info.0;
    if name.contains(char::is_whitespace) {
        return Err(
            ErrorJson::new_400("A channel name may not contain a whitespace!".to_string()).into(),
        );
    }

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
    .await?;

    let channel_obj = Channel {
        id: channel_id,
        name,
        guild_id,
    };

    let event = WsOutboundEvent::ChannelCreate {
        channel: channel_obj.clone(),
    };

    fire_event(&event).await?;

    Ok(crate::Json {
        obj: channel_obj,
        code: 201,
    })
}
