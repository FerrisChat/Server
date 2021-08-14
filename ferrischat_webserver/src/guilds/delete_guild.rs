use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v0/guilds/{guild_id}
pub async fn delete_guild(Path(guild_id): Path<i64>, _: crate::Authorization) -> impl Responder {
    HttpResponse::NoContent()
}
