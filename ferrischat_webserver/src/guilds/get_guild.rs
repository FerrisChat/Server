use actix_web::{web::Path, HttpResponse, Responder};

/// GET /api/v1/guilds/{id}
pub async fn get_guild(Path(guild_id): Path<i64>) -> impl Responder {
    HttpResponse::Found().body("found guild test")
}
