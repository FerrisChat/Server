use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::ChannelCreateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, ModelType};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/{guild_id/channels
pub async fn create_channel(
    _: crate::Authorization,
    channel_info: Json<ChannelCreateJson>,
    req: HttpRequest,
) -> impl Responder {
    let db = get_db_or_fail!();

    let ChannelCreateJson { name } = channel_info.0;

    let node_id = get_node_id!();
    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, node_id);
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    match sqlx::query!(
        "INSERT INTO channels VALUES ($1, $2, $3)",
        bigint_channel_id,
        name,
        bigint_guild_id
    )
    .execute(db)
    .await
    {
        Ok(_) => HttpResponse::Created().json(Channel {
            id: channel_id,
            name,
            guild_id,
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
