use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Invite, Member, NotFoundJson, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;
use serde::Serialize;
use sqlx::types::time::OffsetDateTime;
use sqlx::types::BigDecimal;

const FERRIS_EPOCH: i64 = 1_577_836_800_000;

pub async fn use_invite(
    Path(invite_code): Path<String>,
    crate::Authorization(user_id): crate::Authorization,
) -> Result<crate::Json<impl Serialize>, WebServerError> {
    let bigint_user_id = u128_to_bigdecimal!(user_id);

    let db = get_db_or_fail!();

    let invite = sqlx::query!("SELECT * FROM invites WHERE code = $1", invite_code)
        .fetch_optional(db)
        .await?
        .map(|x| bigdecimal_to_u128!(x))
        .ok_or_else(|| {
            (
                404,
                NotFoundJson {
                    message: format!("Unknown invite with code {}", invite_code),
                },
            )
        })?;

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

        fire_event(format!("invite_{}", guild_id), &event).await?;

        return Err((
            410,
            ferrischat_common::types::Json {
                message: "this invite just disappeared".to_string(),
            },
        )
            .into());
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
        return Err((
            409,
            ferrischat_common::types::Json {
                message: "user has already joined this guild".to_string(),
            },
        )
            .into());
    };

    let member_resp = sqlx::query!(
        "INSERT INTO members VALUES ($1, $2)",
        bigint_user_id,
        invite.guild_id
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
                avatar: None,
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

    fire_event(format!("member_add_{}", user_id), &event).await?;
    Ok(crate::Json {
        obj: member_obj,
        code: 201,
    })
}
