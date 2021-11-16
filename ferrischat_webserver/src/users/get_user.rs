use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{
    Channel, Guild, GuildFlags, InternalServerErrorJson, Member, NotFoundJson, User, UserFlags,
};
use num_traits::cast::ToPrimitive;
use sqlx::Error;

/// GET /api/v0/users/{user_id}
pub async fn get_user(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let user_id = get_item_id!(req, "user_id");
    let db = get_db_or_fail!();
    let bigint_user_id = u128_to_bigdecimal!(user_id);
    let authorized_user = auth.0;
    let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(user) => HttpResponse::Ok().json(User {
                id: user_id,
                name: user.name,
                avatar: None,
                guilds: if authorized_user == user_id {
                    // this code is shit, can probably make it better but i can't figure out the
                    // unsatisfied trait bounds that happens when you get rid of .iter()

                    // note the AS statements here: SQLx cannot properly infer the type due to the `INNER JOIN`
                    // the ! forces the type to `NOT NULL`
                    match sqlx::query!(
                        r#"SELECT id AS "id!", owner_id AS "owner_id!", name AS "name!" FROM guilds INNER JOIN members m on guilds.id = m.guild_id WHERE m.user_id = $1"#,
                        bigint_user_id
                    )
                    .fetch_all(db)
                    .await
                    {
                        Ok(d) => {
                            let mut guilds = Vec::with_capacity(d.len());

                            for x in d {
                                let id_ = x.id.clone()
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
                                    channels: {
                                        let resp = sqlx::query!(
                                            "SELECT * FROM channels WHERE guild_id = $1",
                                            x.id.clone()
                                        )
                                        .fetch_all(db)
                                        .await;

                                        Some(match resp {
                                            Ok(resp) => resp
                                                .iter()
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
                                            Err(e) => {
                                                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                                                    reason: format!("database returned a error: {}", e),
                                                })
                                            }
                                        })
                                    },
                                    flags: GuildFlags::empty(),
                                    members: {
                                        let resp = sqlx::query!("SELECT * FROM members WHERE guild_id = $1", x.id)
                                        .fetch_all(db)
                                        .await;

                                        Some(match resp {
                                            Ok(resp) => {
                                                for x in resp {
                                                    let user = {
                                                        let resp = sqlx::query!("SELECT * FROM users WHERE user_id = $1", x.user_id.clone())
                                                        .fetch_one(db)
                                                        .await;

                                                        match resp {
                                                            Ok(user) => Some(User {
                                                                id: bigdecimal_to_u128!(user.id),
                                                                name: user.name,
                                                                avatar: None,
                                                                guilds: None,
                                                                discriminator: user.discriminator,
                                                                flags: UserFlags::from_bits_truncate(user.flags)
                                                            }),
                                                            Err(e) => {
                                                                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                                                                    reason: format!("database returned a error: {}", e),
                                                                })
                                                            }
                                                        }
                                                    };
                                                    Member {
                                                        user_id: x.user_id.with_scale(0).into_bigint_and_exponent().0.to_u128(),
                                                        user: user,
                                                        guild_id: x.guild_id.with_scale(0).into_bigint_and_exponent().0.to_u128(),
                                                        guild: None,
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                                                    reason: format!("database returned a error: {}", e),
                                                })
                                            }
                                        })
                                    },
                                    roles: None
                                };
                                guilds.push(g);
                            }

                            Some(guilds)
                        },
                        Err(e) => {
                            return HttpResponse::InternalServerError().json(
                                InternalServerErrorJson {
                                    reason: format!("database returned a error: {}", e),
                                },
                            )
                        }
                    }
                } else {
                    None
                },
                discriminator: user.discriminator,
                flags: UserFlags::from_bits_truncate(user.flags),
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "User Not Found".to_string(),
            }),
        },
        Err(e) => {
            if let Error::RowNotFound = e {
                HttpResponse::NotFound().json(NotFoundJson {
                    message: "user not found".to_string(),
                })
            } else {
                HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("database returned a error: {}", e),
                })
            }
        }
    }
}
