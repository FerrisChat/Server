use crate::WebServerError;
use ferrischat_common::types::{BotsOwnedByUser, ErrorJson, User, UserFlags};

/// GET `/v0/users/me/bots`
/// Get all bots owned by the user
pub async fn get_bots_by_user(
    crate::Authorization(auth_user, is_bot): crate::Authorization,
) -> Result<crate::Json<BotsOwnedByUser>, WebServerError> {
    if is_bot {
        return Err(ErrorJson::new_401("Bots cannot create/own bots!".to_string()).into());
    }

    let bigint_user_id = u128_to_bigdecimal!(auth_user);

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
            avatar: user.avatar,
            guilds: None,
            discriminator: user.discriminator,
            flags: UserFlags::from_bits_truncate(user.flags),
            pronouns: user
                .pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
            is_bot: true,
        });
    }
    Ok(crate::Json {
        obj: BotsOwnedByUser { bots },
        code: 200,
    })
}
