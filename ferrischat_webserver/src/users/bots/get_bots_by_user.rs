use crate::WebServerError;
use ferrischat_common::types::{BotsOwnedByUser, User, UserFlags};
use serde::Serialize;

/// GET `/api/v0/users/{user_id}/bots`
/// Get all bots owned by the user
pub async fn get_bots_by_user(
    auth: crate::Authorization,
) -> Result<crate::Json<BotsOwnedByUser>, WebServerError> {
    let bigint_user_id = u128_to_bigdecimal!(auth.0);

    let db = get_db_or_fail!();

    let resp = sqlx::query!(
        "SELECT user_id FROM bots WHERE owner_id = $1",
        bigint_user_id
    )
    .fetch_all(db)
    .await?;

    let mut bots = Vec::with_capacity(resp.len());
    for x in resp {
        let user = sqlx::query!("SELECT * FROM users WHERE id = $1", x.user_id.clone())
            .fetch_one(db)
            .await?;

        let id = bigdecimal_to_u128!(user.id);

        bots.push(User {
            id,
            name: user.name,
            avatar: None,
            guilds: None,
            discriminator: user.discriminator,
            flags: UserFlags::from_bits_truncate(user.flags),
            pronouns: user
                .pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
        })
    }
    Ok(crate::Json {
        obj: BotsOwnedByUser { bots },
        code: 200,
    })
}
