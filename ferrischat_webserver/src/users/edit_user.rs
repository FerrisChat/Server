use crate::WebServerError;
use axum::extract::Json;
use ferrischat_common::request_json::UserUpdateJson;
use ferrischat_common::types::{ErrorJson, User, UserFlags};

/// PATCH `/v0/users/me`
/// Modifies the authenticated user
pub async fn edit_user(
    Json(UserUpdateJson {
        username,
        email,
        avatar,
        password,
        pronouns,
        ..
    }): Json<UserUpdateJson>,
    crate::Authorization(user_id, is_bot): crate::Authorization,
) -> Result<crate::Json<User>, WebServerError> {
    let bigint_user_id = u128_to_bigdecimal!(user_id);
    let db = get_db_or_fail!();

    if let Some(username) = username {
        if username.contains(char::is_whitespace) {
            return Err(ErrorJson::new_400(
                "Your username may not contain a whitespace!".to_string(),
            )
            .into());
        }
        sqlx::query!(
            "UPDATE users SET name = $1 WHERE id = $2",
            username,
            bigint_user_id,
        )
        .execute(db)
        .await?;
    }

    if let Some(avatar) = avatar {
        sqlx::query!(
            "UPDATE users SET avatar = $1 WHERE id = $2",
            avatar,
            bigint_user_id,
        )
        .execute(db)
        .await?;
    }

    if let Some(email) = email {
        sqlx::query!(
            "UPDATE users SET email = $1 WHERE id = $2",
            email,
            bigint_user_id,
        )
        .execute(db)
        .await?;
    }

    if let Some(password) = password {
        let hashed_password = ferrischat_auth::hash(&password).await?;
        sqlx::query!(
            "UPDATE users SET password = $1 WHERE id = $2",
            hashed_password,
            bigint_user_id
        )
        .execute(db)
        .await?;
    }

    if let Some(pronouns) = pronouns {
        sqlx::query!(
            "UPDATE users SET pronouns = $1 WHERE id = $2",
            pronouns as i16,
            bigint_user_id
        )
        .execute(db)
        .await?;
    }

    let user = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("unknown user with id {}", user_id)))?;

    Ok(crate::Json {
        code: 200,
        obj: User {
            id: user_id,
            name: user.name.clone(),
            avatar: user.avatar,
            guilds: None,
            flags: UserFlags::from_bits_truncate(user.flags),
            discriminator: user.discriminator,
            pronouns: user
                .pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
            is_bot,
        },
    })
}
