use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, ModelType, NotFoundJson, Role};
use ferrischat_snowflake_generator::generate_snowflake;

/// DELETE /api/v0/guilds/{guild_id}/members/{user_id}/role/{role_id}
pub async fn remove_member_role(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let role_id = u128_to_bigdecimal!(get_item_id!(req, "role_id"));
    let guild_id = u128_to_bigdecimal!(get_item_id!(req, "guild_id"));
    let user_id = u128_to_bigdecimal!(get_item_id!(req, "user_id"));

    match sqlx::query!(
        "DELETE FROM role_data WHERE guild_id = $1 AND user_id = $2 AND role_id = $3",
        guild_id,
        user_id,
        role_id
    )
    .execute(db)
    .await
    {
        Ok(r) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
