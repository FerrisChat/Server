use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::UserUpdateJson;
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, User, UserFlags};

use tokio::sync::oneshot::channel;

pub async fn edit_user(
    req: HttpRequest,
    user_info: Json<UserUpdateJson>,
    auth: crate::Authorization,
) -> impl Responder {
    let user_id = get_item_id!(req, "user_id");

    if user_id != auth.0 {
        return HttpResponse::Forbidden().finish();
    }

    let bigint_user_id = u128_to_bigdecimal!(user_id);

    let UserUpdateJson {
        username,
        email,
        password,
        avatar: _,
    } = user_info.0;

    let db = get_db_or_fail!();

    if let Some(username) = username {
        let resp = sqlx::query!(
            "UPDATE users SET name = $1 WHERE id = $2",
            username,
            bigint_user_id
        )
        .execute(db)
        .await;
        match resp {
            Ok(_) => (),
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                })
            }
        }
    }

    if let Some(email) = email {
        let resp = sqlx::query!(
            "UPDATE users SET email = $1 WHERE id = $2",
            email,
            bigint_user_id
        )
        .execute(db)
        .await;
        match resp {
            Ok(_) => (),
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                })
            }
        }
    }

    if let Some(password) = password {
        let hashed_password = {
            let hasher = match ferrischat_auth::GLOBAL_HASHER.get() {
                Some(h) => h,
                None => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "Password hasher not found".to_string(),
                    })
                }
            };

            let (tx, rx) = channel();

            let _ = hasher.send((password, tx)).await;
            match rx.await {
                Ok(d) => match d {
                    Ok(s) => s,
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("Failed to hash password: {}", e),
                        })
                    }
                },
                Err(_) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "Other end hung up connection".to_string(),
                    })
                }
            }
        };
        let resp = sqlx::query!(
            "UPDATE users SET password = $1 WHERE id = $2",
            hashed_password,
            bigint_user_id
        )
        .execute(db)
        .await;
        match resp {
            Ok(_) => (),
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                })
            }
        }
    }

    let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(user) => HttpResponse::Ok().json(User {
                id: user_id,
                name: user.name.clone(),
                avatar: None,
                guilds: None,
                flags: UserFlags::from_bits_truncate(user.flags),
                discriminator: user.discriminator,
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "User not found".to_string(),
            }),
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
            })
        }
    }
}
