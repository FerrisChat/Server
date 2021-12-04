use crate::WebServerError;
use axum::extract::{Path, Query};
use ferrischat_common::request_json::GetGuildUrlParams;
use ferrischat_common::types::{Channel, Guild, GuildFlags, Member, ErrorJson, User, UserFlags};
use num_traits::ToPrimitive;
use serde::Serialize;

/// GET `/api/v0/guilds/{guild_id}`
pub async fn get_guild(
    _: crate::Authorization,
    Path(guild_id): Path<u128>,
    Query(params): Query<GetGuildUrlParams>,
) -> Result<crate::Json<Guild>, WebServerError> {
    let db = get_db_or_fail!();
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let guild = sqlx::query!("SELECT * FROM guilds WHERE id = $1", bigint_guild_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| {
            (
                404,
                ErrorJson::new_404(
                    format!("Unknown guild with ID {}", guild_id),
                ),
            )
        })?;

    let channels: Option<Vec<Channel>> = if params.channels.unwrap_or(true) {
        let resp = sqlx::query!(
            "SELECT * FROM channels WHERE guild_id = $1",
            bigint_guild_id
        )
        .fetch_all(db)
        .await?;
        Some(
            resp.iter()
                .filter_map(|x| {
                    Some(Channel {
                        id: x.id.with_scale(0).into_bigint_and_exponent().0.to_u128()?,
                        name: x.name.clone(),
                        guild_id: x
                            .guild_id
                            .with_scale(0)
                            .into_bigint_and_exponent()
                            .0
                            .to_u128()?,
                    })
                })
                .collect(),
        )
    } else {
        None
    };

    let members: Option<Vec<Member>> = if params.members.unwrap_or(false) {
        let resp = sqlx::query!(
            r#"
        SELECT 
               m.*,
               u.name AS name, 
               u.flags AS flags,
               u.discriminator AS discriminator,
               u.pronouns AS pronouns
        FROM members m
            CROSS JOIN LATERAL (
                SELECT * FROM users WHERE id = m.user_id
                )
                as u
        WHERE guild_id = $1
        "#,
            bigint_guild_id
        )
        .fetch_all(db)
        .await?;
        Some(
            resp.iter()
                .filter_map(|x| {
                    let user_id = x
                        .user_id
                        .with_scale(0)
                        .into_bigint_and_exponent()
                        .0
                        .to_u128()?;

                    Some(Member {
                        user_id: Some(user_id),
                        user: Some(User {
                            id: user_id,
                            name: x.name.clone(),
                            avatar: None,
                            guilds: None,
                            flags: UserFlags::from_bits_truncate(x.flags),
                            discriminator: x.discriminator,
                            pronouns: x
                                .pronouns
                                .and_then(ferrischat_common::types::Pronouns::from_i16),
                        }),
                        guild_id: Some(guild_id),
                        guild: None,
                    })
                })
                .collect(),
        )
    } else {
        None
    };

    Ok(crate::Json {
        obj: Guild {
            id: bigdecimal_to_u128!(guild.id),
            owner_id: bigdecimal_to_u128!(guild.owner_id),
            name: guild.name,
            flags: GuildFlags::empty(),
            channels,
            members,
            roles: None,
        },
        code: 200,
    })
}
