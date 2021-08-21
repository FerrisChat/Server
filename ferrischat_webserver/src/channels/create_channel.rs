use actix_web::{HttpResponse, Responder};
use actix_web::web::Json;
use ferrischat_common::types::{Channel, InternalServerErrorJson, ModelType};
use ferrischat_common::request_json::ChannelCreateJson;
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;
use num_traits::FromPrimitive;
use sqlx::types::BigDecimal;

/// POST /api/v0/guilds/{guild_id/channels
pub async fn create_channel(_: crate::Authorization, channel_info: Json<ChannelCreateJson>) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, 0);
    let ChannelCreateJson { name} = channel_info.0;
    match sqlx::query!(
        "INSERT INTO channels VALUES ($1, $2)",
        BigDecimal::from_u128(channel_id),
        name
    )
    .execute(db)
    .await
    {
        Ok(_) => HttpResponse::Created().json(Channel {
            id: channel_id,
            name: name,
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
