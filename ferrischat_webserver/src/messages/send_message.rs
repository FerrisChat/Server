use actix_web::{HttpResponse, Responder};

/// POST /api/v1/channels/{id}/send/
// TODO: add the ID argument
pub async fn create_message() -> impl Responder {
    HttpResponse::Created().body("created message test")
}
