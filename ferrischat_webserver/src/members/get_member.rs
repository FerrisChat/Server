use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, Member, NotFoundJson, User, UserFlags};

/// GET `/api/v0/guilds/{guild_id}/members/{member_id}`
pub async fn get_member(req: HttpRequest) -> impl Responder {
    let guild_id = get_item_id!(req, "guild_id");
    let decimal_guild_id = u128_to_bigdecimal!(guild_id);

    let member_id = get_item_id!(req, "member_id");
    let decimal_member_id = u128_to_bigdecimal!(member_id);

    let db = get_db_or_fail!();

    let resp = sqlx::query!(
        "SELECT * FROM members WHERE user_id = $1 AND guild_id = $2",
        decimal_member_id,
        decimal_guild_id
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(entry) => match entry {
            Some(_member) => {
                let user_resp =
                    sqlx::query!("SELECT * FROM users WHERE id = $1", decimal_member_id)
                        .fetch_optional(db)
                        .await;

                match user_resp {
                    Ok(u) => {
                        let user = match u {
                            Some(u) => Some(User {
                                id: member_id,
                                name: u.name,
                                avatar: None,
                                discriminator: u.discriminator,
                                flags: UserFlags::from_bits_truncate(u.flags),
                                guilds: None,
                            }),
                            None => None,
                        };
                        HttpResponse::Ok().json(Member {
                            user_id: Some(member_id),
                            user,
                            guild_id: Some(guild_id),
                            guild: None,
                        })
                    }
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("database returned an error: {}", e),
                            is_bug: false,
                            link: None,
                        })
                    }
                }
            }
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: format!("Unknown member with id {}", member_id),
                })
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("database returned an error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    }
}
