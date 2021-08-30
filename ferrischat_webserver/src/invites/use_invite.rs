use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::InternalServerErrorJson;
use time;

pub async fn use_invite(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let invite_code = get_item_id!(req, "code");

    let user_id = auth.0;
    let bigint_user_id = u128_to_bigdecimal(user_id);

    let db = get_db_or_fail!();

    let resp = sqlx::query!("SELECT * FROM invites WHERE code = $1", invite_code)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(invite) => {
                let uses = invite.uses + 1;
                let now = {
                    let now = time::OffsetDateTime::now_utc();
                    time::PrimitiveDateTime::new(now.clone().date(), now.time())
                };
                if invite.max_uses < uses {
                    let delete_resp =
                        sqlx::query!("DELETE FROM invites WHERE code = $1", invite_code)
                            .execute(db)
                            .await;

                    match delete_resp {
                        Ok(_) => return HttpResponse::Gone().finish(),
                        Err(e) => {
                            return HttpResponse::InternalServerError().json(
                                InternalServerErrorJson {
                                    reason: format!("DB returned an error: {}", e),
                                },
                            )
                        }
                    }
                }

                if invite.max_age.is_some() {
                    if (now - invite.created_at).whole_seconds() > invite.max_age {
                        let delete_resp =
                            sqlx::query!("DELETE FROM invites WHERE code = $1", invite_code)
                                .execute(db)
                                .await;

                        match delete_resp {
                            Ok(_) => return HttpResponse::Gone().finish(),
                            Err(e) => {
                                return HttpResponse::InternalServerError().json(
                                    InternalServerErrorJson {
                                        reason: format!("DB returned an error: {}", e),
                                    },
                                )
                            }
                        }
                    }
                }

                let member_resp = sqlx::query!(
                    "INSERT INTO members VALUES ($1, $2)",
                    bigint_user_id,
                    invite.guild_id
                )
                .execute(db)
                .await;

                match member_resp {
                    Ok(_) => (),
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("DB returned an error: {}", e),
                        })
                    }
                }

                let uses_resp = sqlx::query!(
                    "UPDATE invites SET uses = $1 WHERE code = $2",
                    uses,
                    invite_code
                )
                .execute(db)
                .await;

                match uses_resp {
                    Ok(_) => (),
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("DB returned an error: {}", e),
                        })
                    }
                }
            }
        },
        None => {
            return HttpResponse::NotFound().finish();
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
            });
        }
    }

    return HttpResponse::Created().finish();
}
