use actix_web::{web::Path, HttpResponse, Responder};

/// GET /api/v1/members/{guild_id}/{member_id}
pub async fn get_member(Path(member_id): Path<i64>) -> impl Responder {
    HttpResponse::Found().body("found member test")
}
