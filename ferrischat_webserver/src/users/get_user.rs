use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{Guild, InternalServerErrorJson, NotFoundJson, User};
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
                guilds: if authorized_user == user_id {
                    // this code is shit, can probably make it better but i can't figure out the
                    // unsatisfied trait bounds that happens when you get rid of .iter()
                    match sqlx::query!(
                        "SELECT * FROM guilds INNER JOIN members m on guilds.id = m.guild_id"
                    )
                    .fetch_all(db)
                    .await
                    {
                        Ok(d) => Some(
                            d.iter()
                                .filter_map(|x| {
                                    Some(Guild {
                                        id: x
                                            .id
                                            .with_scale(0)
                                            .into_bigint_and_exponent()
                                            .0
                                            .to_u128()?,
                                        owner_id: x
                                            .owner_id
                                            .with_scale(0)
                                            .into_bigint_and_exponent()
                                            .0
                                            .to_u128()?,
                                        name: x.name.clone(),
                                        channels: None,
                                        members: None,
                                    })
                                })
                                .collect(),
                        ),
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
                flags: if let Some(f) = user.flags {
                    f as u64
                } else {
                    0
                },
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
