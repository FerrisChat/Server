use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v0/users/{user_id}
pub async fn delete_user(Path(user_id): Path<i64>, _:crate::Authorization) -> impl Responder {
    HttpResponse::NoContent()
}
