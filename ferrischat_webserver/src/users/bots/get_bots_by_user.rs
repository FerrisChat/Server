use actix_web::{HttpResponse, Responder};
use ferrischat_common::types::{BotsOwnedByUser, InternalServerErrorJson, User, UserFlags};

/// GET `/api/v0/users/{user_id}/bots`
/// Get all bots owned by the user
pub async fn get_bots_by_user(auth: crate::Authorization) -> impl Responder {
    let bigint_user_id = u128_to_bigdecimal!(auth.0);

    let db = get_db_or_fail!();

    let resp = sqlx::query!("SELECT * FROM bots WHERE owner_id = $1", bigint_user_id)
        .fetch_all(db)
        .await;
    match resp {
        Ok(resp) => {
            let mut bots = Vec::with_capacity(resp.len());
            for x in resp {
                let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", x.user_id.clone())
                    .fetch_one(db)
                    .await;

                match resp {
                    Ok(user) => bots.push(User {
                        id: bigdecimal_to_u128!(user.id),
                        name: user.name,
                        avatar: None,
                        guilds: None,
                        discriminator: user.discriminator,
                        flags: UserFlags::from_bits_truncate(user.flags),
                        pronouns: user
                            .pronouns
                            .and_then(ferrischat_common::types::Pronouns::from_i16),
                    }),
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("database returned a error: {}", e),
                            is_bug: false,
                            link: None,
                        })
                    }
                }
            }
            HttpResponse::Ok().json(BotsOwnedByUser { bots })
        }
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
            is_bug: false,
            link: None,
        }),
    }
}
