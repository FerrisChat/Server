use actix_web::{web::Path, HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson};
use sqlx::Error;

/// DELETE /api/v0/guilds/{guild_id}
pub async fn delete_guild(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = get_item_id!(req, "guild_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let bigint_user_id = u128_to_bigdecimal!(auth.0);
    match sqlx::query!(
        "DELETE FROM guilds WHERE id = $1 AND owner_id = $2 RETURNING owner_id",
        bigint_channel_id,
        bigint_user_id
    )
    .fetch_optional(db)
    .await
    {
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
        Ok(r) => match r {
            Some(r) => {
                if bigdecimal_to_u128!(r.owner_id) == auth.0 {
                    HttpResponse::NoContent().finish()
                } else {
                    HttpResponse::Forbidden().finish()
                }
            }
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "guild not found".to_string(),
            }),
        },
    }
}
