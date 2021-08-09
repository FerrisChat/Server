use crate::API_VERSION;
use actix_web::{HttpResponse, Responder};
use ferrischat_common::types::{Channel, InternalServerErrorJson};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;
use num_traits::FromPrimitive;
use sqlx::types::BigDecimal;

/// POST /api/v0/guilds/{guild_id/channels
pub async fn create_channel(_:crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = generate_snowflake::<API_VERSION>(0, 0);
    match sqlx::query!(
        "INSERT INTO channels VALUES ($1, $2)",
        BigDecimal::from_u128(channel_id),
        "New Channel"
    )
    .execute(db)
    .await
    {
        Ok(r) => HttpResponse::Created().json(Channel {
            id: channel_id,
            name: "New Channel".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
