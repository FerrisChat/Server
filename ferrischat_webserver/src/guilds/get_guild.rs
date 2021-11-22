use actix_web::{web::Query, HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::GetGuildUrlParams;
use ferrischat_common::types::{
    Channel, Guild, GuildFlags, InternalServerErrorJson, Member, NotFoundJson, User, UserFlags,
};
use num_traits::ToPrimitive;

/// GET `/api/v0/guilds/{guild_id}`
pub async fn get_guild(
    req: HttpRequest,
    _: crate::Authorization,
    params: Query<GetGuildUrlParams>,
) -> impl Responder {
    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let db = get_db_or_fail!();

    let resp = sqlx::query!("SELECT * FROM guilds WHERE id = $1", bigint_guild_id)
        .fetch_optional(db)
        .await;
    let guild = match resp {
        Ok(resp) => match resp {
            Some(g) => g,
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: format!("Unknown guild with id {0}", guild_id),
                })
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("database returned a error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };

    let channels = if params.channels.unwrap_or(true) {
        let resp = sqlx::query!(
            "SELECT * FROM channels WHERE guild_id = $1",
            bigint_guild_id
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
                    is_bug: false,
                    link: None,
                })
            }
        })
    } else {
        None
    };

    let members = if params.members.unwrap_or(false) {
        let resp = sqlx::query!("SELECT m.*, u.name AS name, u.flags AS flags, u.discriminator AS discriminator FROM members m CROSS JOIN LATERAL (SELECT * FROM users WHERE id = m.user_id) as u WHERE guild_id = $1", bigint_guild_id)
            .fetch_all(db)
            .await;
        Some(match resp {
            Ok(resp) => resp
                .iter()
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
                        }),
                        guild_id: Some(guild_id),
                        guild: None,
                    })
                })
                .collect(),
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("database returned a error: {}", e),
                    is_bug: false,
                    link: None,
                })
            }
        })
    } else {
        None
    };

    HttpResponse::Ok().json(Guild {
        id: bigdecimal_to_u128!(guild.id),
        owner_id: bigdecimal_to_u128!(guild.owner_id),
        name: guild.name,
        flags: GuildFlags::empty(),
        channels,
        members,
        roles: None,
    })
}
