use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{ErrorJson, Invite, Member, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::BigDecimal;

const FERRIS_EPOCH: i64 = 1_577_836_800_000;

pub async fn use_invite(
    Path(invite_code): Path<String>,
    crate::Authorization(user_id, is_bot): crate::Authorization,
) -> Result<crate::Json<Member>, WebServerError> {
    if is_bot {
        return Err(ErrorJson::new_401(
            "Bots cannot use invites! They must be invited by the guild owner.".to_string(),
        )
        .into());
    }

    let db = get_db_or_fail!();
    let bigint_user_id = u128_to_bigdecimal!(user_id);

    let invite = sqlx::query!("SELECT * FROM invites WHERE code = $1", invite_code)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown invite with code {}", invite_code)))?;

    let bigint_guild_id: BigDecimal = invite.guild_id;
    let guild_id = bigdecimal_to_u128!(bigint_guild_id);
    let uses = invite.uses + 1;
    let unix_timestamp = OffsetDateTime::now_utc().unix_timestamp();
    let now = unix_timestamp - FERRIS_EPOCH;
    let mut to_delete = false;
    if let Some(max_uses) = invite.max_uses {
        if uses > max_uses.into() {
            to_delete = true;
        }
    }
    if let Some(max_age) = invite.max_age {
        if (now - invite.created_at) > max_age {
            to_delete = true;
        }
    }
    if to_delete {
        sqlx::query!("DELETE FROM invites WHERE code = $1", invite_code)
            .execute(db)
            .await?;

        let invite_obj = Invite {
            code: invite.code.clone(),
            owner_id: bigdecimal_to_u128!(invite.owner_id),
            guild_id,
            created_at: invite.created_at,
            uses,
            max_uses: invite.max_uses,
            max_age: invite.max_age,
        };

        let event = WsOutboundEvent::InviteDelete { invite: invite_obj };

        fire_event(&event).await?;

        return Err(ErrorJson::new("this invite just disappeared".to_string(), 410).into());
    }

    if sqlx::query!(
        r#"SELECT EXISTS(SELECT * FROM members WHERE user_id = $1 AND guild_id = $2) AS "exists!""#,
        bigint_user_id,
        bigint_guild_id
    )
    .fetch_one(db)
    .await?
    .exists
    {
        return Err(ErrorJson::new_409("user has already joined this guild".to_string()).into());
    };

    sqlx::query!(
        "INSERT INTO members VALUES ($1, $2)",
        bigint_user_id,
        bigint_guild_id
    )
    .execute(db)
    .await?;

    let member_obj = Member {
        user_id: Some(user_id),
        user: Some({
            let u = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
                .fetch_one(db)
                .await?;
            User {
                id: user_id,
                name: u.name.clone(),
                avatar: u.avatar,
                guilds: None,
                flags: UserFlags::from_bits_truncate(u.flags),
                discriminator: u.discriminator,
                pronouns: u
                    .pronouns
                    .and_then(ferrischat_common::types::Pronouns::from_i16),
                is_bot,
            }
        }),
        guild_id: Some(guild_id),
        guild: None,
    };

    sqlx::query!(
        "UPDATE invites SET uses = $1 WHERE code = $2",
        uses,
        invite_code
    )
    .execute(db)
    .await?;

    let event = WsOutboundEvent::MemberCreate {
        member: member_obj.clone(),
    };

    fire_event(&event).await?;
    Ok(crate::Json {
        obj: member_obj,
        code: 201,
    })
}
