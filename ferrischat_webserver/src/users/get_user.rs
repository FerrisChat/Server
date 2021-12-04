use crate::WebServerError;
use axum::extract::Path;

use ferrischat_common::types::{Channel, Guild, GuildFlags, Member, NotFoundJson, User, UserFlags};

use num_traits::cast::ToPrimitive;
use serde::Serialize;

/// GET `/api/v0/users/{user_id}`
pub async fn get_user(
    Path(user_id): Path<u128>,
    crate::Authorization(authorized_user): crate::Authorization,
) -> Result<crate::Json<User>, WebServerError> {
    let db = get_db_or_fail!();
    let bigint_user_id = u128_to_bigdecimal!(user_id);

    let user = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| {
            (
                404,
                NotFoundJson {
                    message: format!("Unknown user with ID {}", user_id),
                },
            )
        })?;

    Ok(crate::Json {
        code: 200,
        obj: User {
            id: user_id,
            name: user.name,
            avatar: None,
            guilds: if authorized_user == user_id {
                // this code is shit, can probably make it better but i can't figure out the
                // unsatisfied trait bounds that happens when you get rid of .iter()

                // note the AS statements here: SQLx cannot properly infer the type due to the `INNER JOIN`
                // the ! forces the type to `NOT NULL`

                let d = sqlx::query!(
                    r#"
                        SELECT 
                            id AS "id!",
                            owner_id AS "owner_id!",
                            name AS "name!"
                        FROM 
                            guilds
                        INNER JOIN
                            members m ON guilds.id = m.guild_id
                        WHERE
                            m.user_id = $1
                    "#,
                    bigint_user_id,
                )
                .fetch_all(db)
                .await?;

                let mut guilds = Vec::with_capacity(d.len());

                for x in d {
                    let id_ =
                        x.id.clone()
                            .with_scale(0)
                            .into_bigint_and_exponent()
                            .0
                            .to_u128();

                    let id = match id_ {
                        Some(id) => id,
                        None => continue,
                    };

                    let owner_id_ = x
                        .owner_id
                        .with_scale(0)
                        .into_bigint_and_exponent()
                        .0
                        .to_u128();

                    let owner_id = match owner_id_ {
                        Some(owner_id) => owner_id,
                        None => continue,
                    };

                    let g = Guild {
                        id,
                        owner_id,
                        name: x.name.clone(),
                        channels: Some(
                            sqlx::query!(
                                "SELECT * FROM channels WHERE guild_id = $1",
                                x.id.clone()
                            )
                            .fetch_all(db)
                            .await?
                            .iter()
                            .filter_map(|x| {
                                Some(Channel {
                                    id: x
                                        .id
                                        .with_scale(0)
                                        .into_bigint_and_exponent()
                                        .0
                                        .to_u128()?,
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
                        ),
                        flags: GuildFlags::empty(),
                        members: {
                            let resp =
                                sqlx::query!("SELECT * FROM members WHERE guild_id = $1", x.id)
                                    .fetch_all(db)
                                    .await?;

                            Some({
                                let mut members = Vec::with_capacity(resp.len());

                                for x in resp {
                                    let user = {
                                        let user = sqlx::query!(
                                            "SELECT * FROM users WHERE id = $1",
                                            x.user_id.clone()
                                        )
                                        .fetch_one(db)
                                        .await?;

                                        Some(User {
                                            id: bigdecimal_to_u128!(user.id),
                                            name: user.name,
                                            avatar: None,
                                            guilds: None,
                                            discriminator: user.discriminator,
                                            flags: UserFlags::from_bits_truncate(user.flags),
                                            pronouns: user.pronouns.and_then(
                                                ferrischat_common::types::Pronouns::from_i16,
                                            ),
                                        })
                                    };

                                    let member = Member {
                                        user_id: x
                                            .user_id
                                            .with_scale(0)
                                            .into_bigint_and_exponent()
                                            .0
                                            .to_u128(),
                                        user,
                                        guild_id: x
                                            .guild_id
                                            .with_scale(0)
                                            .into_bigint_and_exponent()
                                            .0
                                            .to_u128(),
                                        guild: None,
                                    };

                                    members.push(member);
                                }
                                members
                            })
                        },
                        roles: None,
                    };
                    guilds.push(g);
                }

                Some(guilds)
            } else {
                None
            },
            discriminator: user.discriminator,
            flags: UserFlags::from_bits_truncate(user.flags),
            pronouns: user
                .pronouns
                .and_then(ferrischat_common::types::Pronouns::from_i16),
        },
    })
}
