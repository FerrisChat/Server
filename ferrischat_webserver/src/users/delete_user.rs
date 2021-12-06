use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::ErrorJson;

/// DELETE `/api/v0/users/{user_id}`
/// Deletes the authenticated user
pub async fn delete_user(
    Path(user_id): Path<u128>,
    auth: crate::Authorization,
) -> Result<http::StatusCode, WebServerError> {
    if user_id != auth.0 {
        return Err(ErrorJson::new_403("this account is not yours".to_string()).into());
    }

    let bigint_user_id = u128_to_bigdecimal!(user_id);
    let db = get_db_or_fail!();

    // Drop the user.
    sqlx::query!(
        "DELETE FROM users WHERE id = $1 RETURNING (id)",
        bigint_user_id,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ErrorJson::new_404("account not found".to_string()))?;

    Ok(http::StatusCode::NO_CONTENT)
}
