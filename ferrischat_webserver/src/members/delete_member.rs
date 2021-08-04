use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v1/members/{guild_id}/{member_id}
pub async fn delete_member(Path(member_id): Path<i64>) -> impl Responder {
    HttpResponse::NoContent().body("deleted member test")
}
