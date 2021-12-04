use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Json, ModelType};
use ferrischat_snowflake_generator::generate_snowflake;
use serde::Serialize;

/// POST `/api/v0/guilds/{guild_id}/members/{user_id}/role/{role_id}`
pub async fn add_member_role(
    Path((guild_id, user_id, role_id)): Path<(u128, u128, u128)>,
    _: crate::Authorization,
) -> Result<crate::Json<Json>, WebServerError> {
    let db = get_db_or_fail!();

    let guild_id = u128_to_bigdecimal!(guild_id);
    let user_id = u128_to_bigdecimal!(user_id);
    let role_id = u128_to_bigdecimal!(role_id);
    let internal_id = u128_to_bigdecimal!(generate_snowflake::<0>(
        ModelType::InternalUse as u8,
        get_node_id!()
    ));

    sqlx::query!(
        "INSERT INTO role_data VALUES ($1, $2, $3, $4)",
        internal_id,
        guild_id,
        user_id,
        role_id
    )
    .execute(db)
    .await?;

    Ok(crate::Json {
        obj: Json {
            message: "role added to user successfully".to_string(),
        },
        code: 200,
    })
}
