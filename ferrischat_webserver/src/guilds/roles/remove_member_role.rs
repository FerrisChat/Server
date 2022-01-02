use crate::WebServerError;
use axum::extract::Path;
use http::StatusCode;

/// DELETE `/v0/guilds/{guild_id}/members/{user_id}/role/{role_id}`
pub async fn remove_member_role(
    Path((guild_id, user_id, role_id)): Path<(u128, u128, u128)>,
    _: crate::Authorization,
) -> Result<StatusCode, WebServerError> {
    let db = get_db_or_fail!();
    let guild_id = u128_to_bigdecimal!(guild_id);
    let user_id = u128_to_bigdecimal!(user_id);
    let role_id = u128_to_bigdecimal!(role_id);

    sqlx::query!(
        "DELETE FROM role_data WHERE guild_id = $1 AND user_id = $2 AND role_id = $3",
        guild_id,
        user_id,
        role_id
    )
    .execute(db)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
