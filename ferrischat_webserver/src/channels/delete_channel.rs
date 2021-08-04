use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v1/channels/{id}
pub async fn delete_channel(Path(channel_id): Path<i64>) -> impl Responder {
    HttpResponse::NoContent().body("deleted channel test")
}
