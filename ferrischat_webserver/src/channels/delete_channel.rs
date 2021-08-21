use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson};
use sqlx::Error;

/// DELETE /api/v0/guilds/{guild_id/channels/{channel_id}
pub async fn delete_channel(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = get_item_id!(req, "channel_id");
    let guild_id = get_item_id!(req, "guild_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    if let Err(e) = sqlx::query!("DELETE FROM channels WHERE id = $1 AND guild_id = $2", bigint_channel_id, bigint_guild_id)
        .execute(db)
        .await
    {
        if let Error::RowNotFound = e {
            HttpResponse::NotFound().json(NotFoundJson {
                message: "channel not found".to_string(),
            })
        } else {
            HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned a error: {}", e),
            })
        }
    } else {
        HttpResponse::NoContent().finish()
    }
}
