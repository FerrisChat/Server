use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v1/users/{id}
pub async fn delete_user(Path(user_id): Path<i64>) -> impl Responder {
    HttpResponse::NoContent()
}
