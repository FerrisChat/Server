use actix_web::{HttpRequest, HttpResponse, Responder};

use ferrischat_common::types::{
    BotsOwnedByUser, InternalServerErrorJson, User, UserFlags,
};

/// GET `/api/v0/users/{user_id}/bots`
/// Get all bots owned by the user
pub async fn get_bots_by_user(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let bigint_user_id = u128_to_bigdecimal!(auth.0);

    let db = get_db_or_fail!();

    let resp = sqlx::query!("SELECT * FROM bots WHERE owner_id = $1", bigint_user_id)
        .fetch_all(db)
        .await;

    let bots = match resp {
        Ok(resp) => resp
            .iter()
            .filter_map(|x| {
                let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", x.bot_id)
                    .fetch_one(db)
                    .await;

                match resp {
                    Ok(resp) => User {
                        id: resp.id,
                        name: resp.name,
                        avatar: None,
                        guilds: None,
                        discriminator: resp.discriminator,
                        flags: UserFlags::from_bits_truncate(resp.flags),
                        pronouns: None,
                    },
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("database returned a error: {}", e),
                            is_bug: false,
                            link: None,
                        });
                    }
                }
            })
            .collect(),
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("database returned a error: {}", e),
                is_bug: false,
                link: None,
            });
        }
    };

    return HttpResponse::Ok().json(BotsOwnedByUser { bots });
}
