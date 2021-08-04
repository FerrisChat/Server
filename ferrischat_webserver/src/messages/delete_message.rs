use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v1/messages/{id}
pub async fn delete_message(Path(message_id): Path<i64>) -> impl Responder {
    HttpResponse::NoContent().body("deleted message test")
}
