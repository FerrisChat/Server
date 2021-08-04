use actix_web::{web::Path, HttpResponse, Responder};

/// GET /api/v1/users/{id}
pub async fn get_user(Path(user_id): Path<i64>) -> impl Responder {
    HttpResponse::Found().body("found user test")
}
