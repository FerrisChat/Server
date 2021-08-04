use actix_web::{web::Path, HttpResponse, Responder};

/// GET /api/v1/channels/{id}
pub async fn get_channel(Path(channel_id): Path<i64>) -> impl Responder {
    HttpResponse::Found().body("found channel test")
}
