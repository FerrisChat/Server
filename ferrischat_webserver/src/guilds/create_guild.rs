use actix_web::web::Json;
use actix_web::{HttpResponse, Responder};
use ferrischat_common::request_json::GuildCreateJson;
use ferrischat_common::types::{Guild, InternalServerErrorJson, ModelType};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;
use num_traits::FromPrimitive;
use sqlx::types::BigDecimal;

/// POST /api/v0/guilds/
pub async fn create_guild(
    auth: crate::Authorization,
    guild_info: Json<GuildCreateJson>,
) -> impl Responder {
    let db = get_db_or_fail!();
    let guild_id = generate_snowflake::<0>(ModelType::Guild as u8, 0);
    let GuildCreateJson { name } = guild_info.0;
    match sqlx::query!(
        "INSERT INTO guilds VALUES ($1, $2, $3, 0, 0)",
        BigDecimal::from_u128(guild_id),
        BigDecimal::from_u128(auth.0),
        name
    )
    .execute(db)
    .await
    {
        Ok(_) => HttpResponse::Created().json(Guild {
            id: guild_id,
            owner_id: auth.0,
            name,
            channels: None,
            members: None,
        }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
