use actix_web::{HttpResponse, Responder};

/// POST /api/v1/guilds/
pub async fn create_guild() -> impl Responder {
    HttpResponse::Created().body("created guild test")
}
