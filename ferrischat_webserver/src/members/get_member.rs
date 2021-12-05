use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{ErrorJson, Member, User, UserFlags};

/// GET `/api/v0/guilds/{guild_id}/members/{member_id}`
pub async fn get_member(
    Path((guild_id, member_id)): Path<(u128, u128)>,
    auth: crate::Authorization,
) -> Result<crate::Json<Member>, WebServerError> {
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let bigint_member_id = u128_to_bigdecimal!(member_id);

    let db = get_db_or_fail!();

    let member_resp = sqlx::query!(
        "SELECT * FROM members WHERE user_id = $1 AND guild_id = $2",
        bigint_member_id,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ErrorJson::new_404(format!("Unknown member with ID {}", member_id)))?;

    let user = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_member_id)
        .fetch_optional(db)
        .await?
        .map(|u| User {
            id: member_id,
            name: u.name,
            avatar: u.avatar,
            discriminator: u.discriminator,
            flags: UserFlags::from_bits_truncate(u.flags),
            guilds: None,
            pronouns: u
                .pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
        });

    let member_obj = Member {
        user_id: Some(member_id),
        user,
        guild_id: Some(guild_id),
        guild: None,
    };

    Ok(crate::Json {
        obj: member_obj,
        code: 200,
    })
}
