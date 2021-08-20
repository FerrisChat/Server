use actix_web::{HttpResponse, Responder};
use ferrischat_common::types::{Channel, InternalServerErrorJson, ModelType};
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/{guild_id/channels
pub async fn create_channel(_: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, 0);
    match sqlx::query!(
        "INSERT INTO channels VALUES ($1, $2)",
        u128_to_bigdecimal!(channel_id),
        "New Channel"
    )
    .execute(db)
    .await
    {
        Ok(_) => HttpResponse::Created().json(Channel {
            id: channel_id,
            name: "New Channel".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
