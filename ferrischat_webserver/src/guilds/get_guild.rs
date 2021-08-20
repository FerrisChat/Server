use actix_web::{HttpResponse, Responder, HttpRequest};
use ferrischat_common::types::{Guild, InternalServerErrorJson};
use ferrischat_macros::{bigdecimal_to_u128, get_db_or_fail};
use num_traits::cast::ToPrimitive;
use sqlx::types::BigDecimal;

/// GET /api/v0/guilds/{guild_id}
pub async fn get_guild(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let guild_id = u128_to_bigdecimal!(get_item_id!(req, "guild_id"));
    let db = get_db_or_fail!();
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
