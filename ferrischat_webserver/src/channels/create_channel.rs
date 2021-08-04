use actix_web::{HttpResponse, Responder};

/// POST /api/v1/channels/
pub async fn create_channel() -> impl Responder {
    HttpResponse::Created().body("created channel test")
}
