use crate::WebServerError;
use axum::extract::Path;
use axum::Json;
use ferrischat_common::request_json::BotUpdateJson;
use ferrischat_common::types::{ErrorJson, User, UserFlags};
use sqlx::types::BigDecimal;

/// PATCH `/v0/users/me/bots/{bot_id}`
/// Edits the bot with the attached payload
pub async fn edit_bot(
    Path((_, bot_id)): Path<(u128, u128)>,
    Json(BotUpdateJson {
        username, avatar, ..
    }): Json<BotUpdateJson>,
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
    .ok_or_else(|| ErrorJson::new_404(format!("Unknown bot with ID {}", bot_id)))?
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
        .await?;
    }

    if let Some(avatar) = avatar {
        sqlx::query!(
            "UPDATE users SET avatar = $1 WHERE id = $2",
            avatar,
            bigint_bot_id,
        )
        .execute(db)
        .await?;
    }

    let user = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_bot_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown bot with ID {}", bot_id)))?;
    Ok(crate::Json::new(
        User {
            id: bot_id,
            name: user.name.clone(),
            avatar: None,
            guilds: None,
            flags: UserFlags::from_bits_truncate(user.flags),
            discriminator: user.discriminator,
            pronouns: None,
        },
        200,
    ))
}
