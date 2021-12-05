use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Invite, ErrorJson};
use serde::Serialize;

/// GET `/api/v0/guilds/{guild_id}/invites`
pub async fn get_guild_invites(
    Path(guild_id): Path<u128>,
    crate::Authorization(authorized_user): crate::Authorization,
) -> Result<crate::Json<Vec<Invite>>, WebServerError> {
    let db = get_db_or_fail!();
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let bigint_authed_user = u128_to_bigdecimal!(authorized_user);

    if sqlx::query!(
        "SELECT * FROM members WHERE user_id = $1 AND guild_id = $2",
        bigint_authed_user,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await?
    .is_none()
    {
        return Err(ErrorJson::new_403(
            "you are not a member of this guild".to_string(),
        ).into());
    }

    let res_invites = sqlx::query!("SELECT * FROM invites WHERE guild_id = $1", bigint_guild_id)
        .fetch_all(db)
        .await?
        .iter()
        .map(|invite| {
            Ok(Invite {
                code: invite.code,
                owner_id: bigdecimal_to_u128!(invite.owner_id),
                guild_id: bigdecimal_to_u128!(invite.guild_id),
                created_at: invite.created_at,
                uses: invite.uses,
                max_uses: invite.max_uses,
                max_age: invite.max_age,
            })
        });

    let mut invites = Vec::with_capacity(res_invites.len());
    for invite in res_invites {
        invites.push(invite?);
    }

    Ok(crate::Json {
        obj: invites,
        code: 200,
    })
}
