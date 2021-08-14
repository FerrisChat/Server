use actix_web::{HttpResponse, Responder};

/// POST /api/v0/guilds/{guild_id}/messages
// TODO: add the ID argument
pub async fn create_message() -> impl Responder {
    HttpResponse::Created().body("created message test")
}
