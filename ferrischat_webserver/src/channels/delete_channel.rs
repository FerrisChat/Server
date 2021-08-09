use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v0/guilds/{guild_id/channels/{channel_id}
pub async fn delete_channel(Path(channel_id): Path<i64>, _:crate::Authorization) -> impl Responder {
    HttpResponse::NoContent()
}
