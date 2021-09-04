use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson};

/// DELETE /api/v0/guilds/{guild_id}/members/{member_id}
pub async fn delete_member(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let guild_id = {
        let raw = get_item_id!(req, "guild_id");
        u128_to_bigdecimal!(raw)
    };
    let member_id = {
        let raw = get_item_id!(req, "member_id");
        u128_to_bigdecimal!(raw)
    };

    let db = get_db_or_fail!();

    let resp = sqlx::query!(
        "DELETE FROM members WHERE user_id = $1 AND guild_id = $2 RETURNING (user_id)",
        member_id,
        guild_id
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(r) => match r {
            Some(_) => HttpResponse::NoContent().finish(),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Member not found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("Database responded with an error: {}", e),
        }),
    }
}
