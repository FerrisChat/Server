use actix_web::{HttpResponse, Responder};

/// POST /api/v1/members/{guild_id}/
pub async fn create_member() -> impl Responder {
    HttpResponse::Created().body("created guild member test")
}
