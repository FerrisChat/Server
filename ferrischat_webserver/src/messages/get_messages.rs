use actix_web::{web::Path, HttpResponse, Responder};

/// GET /api/v1/messages/{id}
pub async fn get_message(Path(message_id): Path<i64>) -> impl Responder {
    HttpResponse::Found().body("found message test")
}
