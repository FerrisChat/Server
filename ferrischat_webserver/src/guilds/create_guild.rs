use crate::API_VERSION;
use actix_web::{HttpResponse, Responder};
use ferrischat_common::types::{Guild, InternalServerErrorJson};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;
use num_traits::FromPrimitive;
use sqlx::types::BigDecimal;

/// POST /api/v1/guilds/
pub async fn create_guild() -> impl Responder {
    let db = get_db_or_fail!();
    let guild_id = generate_snowflake::<0>(0, 0);
    match sqlx::query!(
        "INSERT INTO guilds VALUES ($1, $2, $3, 0, 0)",
        BigDecimal::from_u128(guild_id),
        0,
        "New Guild"
    )
    .execute(db)
    .await
    {
        Ok(r) => HttpResponse::Created().json(Guild {
            id: guild_id,
            owner_id: 0,
            name: "New Guild".to_string(),
            channels: None,
            members: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
