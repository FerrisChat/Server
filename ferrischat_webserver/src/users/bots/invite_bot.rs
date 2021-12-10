use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{ErrorJson, Member, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;

/// POST /v0/bots/{bot_id}/add/{guild_id}
pub async fn invite_bot(
    Path((bot_id, guild_id)): Path<(u128, u128)>,
    auth: crate::Authorization,
) -> Result<crate::Json<Member>, WebServerError> {
    let bigint_bot_id = u128_to_bigdecimal!(bot_id);
    let db = get_db_or_fail!();
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let guild = sqlx::query!("SELECT * FROM guilds WHERE id = $1", bigint_guild_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown guild with ID {}", guild_id)))?;

    let guild_owner = bigdecimal_to_u128!(guild.owner_id);
    if guild_owner != auth.0 {
        return Err(ErrorJson::new_403("You don't own this guild!".to_string()).into());
    }

    if sqlx::query!(
        r#"SELECT EXISTS(SELECT * FROM members WHERE user_id = $1 AND guild_id = $2) AS "exists!""#,
        bigint_bot_id,
        bigint_guild_id
    )
    .fetch_one(db)
    .await?
    .exists
    {
        return Err(ErrorJson::new_409("bot has already joined this guild".to_string()).into());
    };

    sqlx::query!(
        "INSERT INTO members VALUES ($1, $2)",
        bigint_bot_id,
        bigint_guild_id
    )
    .execute(db)
    .await?;

    let member_obj = Member {
        user_id: Some(bot_id),
        user: Some({
            let u = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_bot_id)
                .fetch_one(db)
                .await?;
            User {
                id: bot_id,
                name: u.name.clone(),
                avatar: u.avatar,
                guilds: None,
                flags: UserFlags::from_bits_truncate(u.flags),
                discriminator: u.discriminator,
                pronouns: u
                    .pronouns
                    .and_then(ferrischat_common::types::Pronouns::from_i16),
            }
        }),
        guild_id: Some(guild_id),
        guild: None,
    };

    let event = WsOutboundEvent::MemberCreate {
        member: member_obj.clone(),
    };

    fire_event(format!("member_add_{}", bot_id), &event).await?;
    Ok(crate::Json {
        obj: member_obj,
        code: 201,
    })
}
