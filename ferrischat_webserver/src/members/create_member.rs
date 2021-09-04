use actix_web::{HttpResponse, HttpRequest, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson};

/// POST /api/v0/guilds/{guild_id}/members
pub async fn create_member(
    auth: crate::Authorization,
    req: HttpRequest,
) -> impl Responder {
    let guild_id = {
        let raw = get_item_id!(req, "guild_id");
        u128_to_bigdecimal!(raw)
    };
    let member_id = {
        let raw = auth.0;
        u128_to_bigdecimal!(raw)
    }
    
    let db = get_db_or_fail!();

    let resp = sqlx::query!("INSERT INTO members (id, guild_id) VALUES ($1, $2) RETURNING (id)", member_id, guild_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(r) => match r {
            // Currently returns this but member object could work too
            Some(_) => HttpResponse::Created().body("Created guild member"),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Member not found"
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("Database responded with an error: {}", e)
        })
    }
}
