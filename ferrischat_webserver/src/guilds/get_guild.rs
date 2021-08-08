use actix_web::{
    web::{Json, Path},
    HttpResponse, Responder,
};
use ferrischat_common::types::{Guild, InternalServerErrorJson};
use ferrischat_macros::{bigdecimal_to_u128, get_db_or_fail};
use num_traits::cast::ToPrimitive;
use sqlx::types::BigDecimal;

/// GET /api/v1/guilds/{id}
pub async fn get_guild(Path(guild_id): Path<i64>, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let guild_id = BigDecimal::from(guild_id);
    let resp = sqlx::query!("SELECT * FROM guilds WHERE id = $1", guild_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(guild) => HttpResponse::Ok().json(Guild {
                id: bigdecimal_to_u128!(guild.id),
                owner_id: bigdecimal_to_u128!(guild.owner_id),
                name: guild.name,
                channels: None,
                members: None,
            }),
            None => HttpResponse::NotFound().finish(),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
