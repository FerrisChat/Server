use actix_web::{HttpResponse, Responder};

/// POST /api/v1/users/
pub async fn create_user() -> impl Responder {
    HttpResponse::Created().body("created user test")
}
