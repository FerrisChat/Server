use actix_web::{web::Path, HttpResponse, Responder};

/// DELETE /api/v1/guilds/{id}
pub async fn delete_guild(Path(guild_id): Path<i64>) -> impl Responder {
    HttpResponse::NoContent().body("deleted guild test")
}
