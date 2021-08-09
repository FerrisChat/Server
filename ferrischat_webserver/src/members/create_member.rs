use actix_web::{HttpResponse, Responder};

/// POST /api/v0/guilds/{guild_id}/members
pub async fn create_member() -> impl Responder {
    HttpResponse::Created().body("created guild member test")
}
