use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{Guild, InternalServerErrorJson, NotFoundJson};

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
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Guild Not Found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
