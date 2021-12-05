use crate::WebServerError;
use axum::extract::Path;
use axum::Json;
use ferrischat_common::request_json::BotUpdateJson;
use ferrischat_common::types::{ErrorJson, User, UserFlags};
use serde::Serialize;
use sqlx::types::BigDecimal;

/// PATCH `/api/v0/users/{user_id}/bots/{bot_id}`
/// Edits the bot with the attached payload
pub async fn edit_bot(
    Path((user_id, bot_id)): Path<(u128, u128)>,
    Json(BotUpdateJson { username }): Json<BotUpdateJson>,
    auth: crate::Authorization,
) -> Result<crate::Json<User>, WebServerError> {
    let bigint_bot_id = u128_to_bigdecimal!(bot_id);

    let db = get_db_or_fail!();

    let bigint_owner_id: BigDecimal = sqlx::query!(
        "SELECT owner_id FROM bots WHERE user_id = $1",
        bigint_bot_id,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        (
            404,
            ErrorJson::new_404(format!("Unknown bot with ID {}", bot_id)),
        )
            .into()
    })?
    .owner_id;

    let owner_id = bigdecimal_to_u128!(bigint_owner_id);

    if owner_id != auth.0 {
        return Err(ErrorJson::new_403("you are not the owner of this bot".to_string()).into());
    }

    if let Some(username) = username {
        sqlx::query!(
            "UPDATE users SET name = $1 WHERE id = $2",
            username,
            bigint_bot_id
        )
        .execute(db)
        .await?
    }

    sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_bot_id)
        .fetch_optional(db)
        .await?
        .map(|user| User {
            id: bot_id,
            name: user.name.clone(),
            avatar: None,
            guilds: None,
            flags: UserFlags::from_bits_truncate(user.flags),
            discriminator: user.discriminator,
            pronouns: None,
        })
        .ok_or_else(|| {
            (
                404,
                ErrorJson::new_404(format!("Unknown bot with ID {}", bot_id)),
            )
                .into()
        })?
}
