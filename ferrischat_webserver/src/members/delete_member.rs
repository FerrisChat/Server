use actix_web::{HttpRequest, HttpResponse, Responder};

/// DELETE /api/v0/guilds/{guild_id}/members/{member_id}
pub async fn delete_member(req: HttpRequest) -> impl Responder {
    HttpResponse::NoContent()
}
