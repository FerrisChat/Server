use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{
    Member, User, UserFlags,
};
use serde::Serialize;

/// GET `/api/v0/guilds/{guild_id}/members/{member_id}`
pub async fn get_member(Path((guild_id, member_id)): Path<(u128, u128)>, auth: crate::Authorization) -> Result<crate::Json<Member>, WebServerError<impl Serialize>> {
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let bigint_member_id = u128_to_bigdecimal!(member_id);

    let db = get_db_or_fail!();

    let member_resp = sqlx::query!(
        "SELECT * FROM members WHERE user_id = $1 AND guild_id = $2",
        decimal_member_id,
        decimal_guild_id
    )
        .fetch_optional(db)
        .await
        .map_err(|e| WebServerError::Database(e))?;

    let user_resp = sqlx::query!("SELECT * FROM users WHERE id = $1", decimal_member_id)
        .fetch_optional(db)
        .await
        .map_err(|e| WebServerError::Database(e))?;

    let user_obj = Some(User {
        id: member_id,
        name: user_resp.name,
        avatar: user_resp.avatar,
        discriminator: user_resp.discriminator,
        flags: UserFlags::from_bits_truncate(user_resp.flags),
        guilds: None,
        pronouns: user_resp.pronouns,
    });

    let member_obj = Member {
        user_id: Some(member_id),
        user: user_obj,
        guild_id: Some(guild_id),
        guild: None,
    };

    Ok(crate::Json {
        obj: member_obj,
        code: 200,
    })
}
