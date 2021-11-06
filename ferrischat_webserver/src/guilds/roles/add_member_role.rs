use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, ModelType, NotFoundJson, Role};
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/{guild_id}/members/{user_id}/role/{role_id}
pub async fn add_member_role(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let role_id = u128_to_bigdecimal!(get_item_id!(req, "role_id"));
    let guild_id = u128_to_bigdecimal!(get_item_id!(req, "guild_id"));
    let user_id = u128_to_bigdecimal!(get_item_id!(req, "user_id"));
    let internal_id = u128_to_bigdecimal!(generate_snowflake::<0>(
        ModelType::InternalUse as u8,
        get_node_id!()
    ));

    match sqlx::query!(
        "INSERT INTO role_data VALUES ($1, $2, $3, $4)",
        internal_id,
        guild_id,
        user_id,
        role_id
    )
    .execute(db)
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
