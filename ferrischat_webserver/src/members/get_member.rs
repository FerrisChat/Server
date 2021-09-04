use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, Member, NotFoundJson, User};

/// GET /api/v0/guilds/{guild_id}/members/{member_id}
pub async fn get_member(req: HttpRequest) -> impl Responder {
    let guild_id = {
        let raw = get_item_id!(req, "guild_id");
        u128_to_bigdecimal!(raw)
    };
    let member_id = {
        let raw = get_item_id!(req, "member_id");
        u128_to_bigdecimal!(raw)
    };

    let db = get_db_or_fail!();

    let resp = sqlx::query!(
        "SELECT * FROM members WHERE user_id = $1 AND guild_id = $2",
        member_id,
        guild_id
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(entry) => match entry {
            Some(member) => {
                let user_resp = sqlx::query!("SELECT * FROM users WHERE id = $1", member_id)
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
                                flags: u.flags,
                            }),
                            None => None,
                        };
                        HttpResponse::Ok().json(Member {
                            user_id: member_id,
                            user: user,
                            guild_id,
                            guild: None,
                        })
                    }
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("database returned an error: {}", e),
                        })
                    }
                }
            }
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: "Member not found".to_string(),
                })
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("database returned an error: {}", e),
            })
        }
    }
}
