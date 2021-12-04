use crate::WebServerError;
use axum::extract::{Json, Path};
use ferrischat_common::request_json::UserUpdateJson;
use ferrischat_common::types::{ErrorJson, User, UserFlags};
use serde::Serialize;
use tokio::sync::oneshot::channel;

/// PATCH `/api/v0/users/{user_id}`
/// Modifies the authenticated user
pub async fn edit_user(
    Path(user_id): Path<u128>,
    Json(UserUpdateJson {
        username,
        email,
        password,
        pronouns,
        ..
    }): Json<UserUpdateJson>,
    auth: crate::Authorization,
) -> Result<crate::Json<User>, WebServerError> {
    if user_id != auth.0 {
        return Err((
            403,
            ErrorJson::new_403(
                "this account is not yours".to_string(),
            ),
        )
            .into());
    }

    let bigint_user_id = u128_to_bigdecimal!(user_id);
    let db = get_db_or_fail!();

    if let Some(username) = username {
        sqlx::query!(
            "UPDATE users SET name = $1 WHERE id = $2",
            username,
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
        let hashed_password = {
            let (tx, rx) = channel();
            ferrischat_auth::GLOBAL_HASHER
                .get()
                .ok_or(WebServerError::MissingHasher)?
                .send((password, tx))
                .await
                .map_err(|_| {
                    (
                        500,
                        ErrorJson::new_500(
                            "Password hasher has hung up connection".to_string(),
                            false,
                            None,
                        ),
                    )
                        .into()
                })?;
            rx.await
            .unwrap_or_else(|e| {
                unreachable!(
                    "failed to receive value from channel despite value being sent earlier on: {}",
                    e
                )
            })
            .map_err(|e| {
                (
                    500,
                    ErrorJson::new_500(
                        format!("failed to hash token: {}", e),
                        true,
                        Some(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                             labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+failed+to+hash+token"
                                .to_string(),
                        ),
                    ),
                )
                    .into()
            })?
        };
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
        .ok_or_else(|| {
            (
                404,
                ErrorJson::new_404(
                    format!("unknown user with id {}", user_id),
                ),
            )
        })?;

    Ok(crate::Json {
        code: 200,
        obj: User {
            id: user_id,
            name: user.name.clone(),
            avatar: None,
            guilds: None,
            flags: UserFlags::from_bits_truncate(user.flags),
            discriminator: user.discriminator,
            pronouns: user
                .pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
        },
    })
}
