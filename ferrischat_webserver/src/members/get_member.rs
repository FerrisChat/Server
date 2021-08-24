use actix_web::{HttpRequest, HttpResponse, Responder};

/// GET /api/v0/guilds/{guild_id}/members/{member_id}
pub async fn get_member(req: HttpRequest) -> impl Responder {
    HttpResponse::Found().body("found member test")
}
