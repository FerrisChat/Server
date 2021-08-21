use actix_web::web::Json;
use actix_web::{HttpResponse, Responder};
use ferrischat_common::request_json::ChannelCreateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, ModelType};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/{guild_id/channels
pub async fn create_channel(
    _: crate::Authorization,
    channel_info: Json<ChannelCreateJson>,
) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, 0);
    let ChannelCreateJson { name } = channel_info.0;
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    match sqlx::query!(
        "INSERT INTO channels VALUES ($1, $2)",
        bigint_channel_id,
        name
    )
    .execute(db)
    .await
    {
        Ok(_) => HttpResponse::Created().json(Channel {
            id: channel_id,
            name,
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
