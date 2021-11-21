use crate::ws::{fire_event, WsEventError};
use ferrischat_common::ws::WsOutboundEvent;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{
    InternalServerErrorJson, Invite, Member, NotFoundJson, User, UserFlags,
};
use sqlx::types::time::OffsetDateTime;

const FERRIS_EPOCH: i64 = 1_577_836_800_000;

pub async fn use_invite(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let invite_code = {
        match req.match_info().get("code") {
            Some(invite_code) => match invite_code.parse::<String>() {
                Ok(invite_code) => invite_code,
                Err(_) => {
                    return HttpResponse::BadRequest().json(InternalServerErrorJson {
                        reason: "Failed to parse invite code as String".to_string(),
                    })
                }
            },
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "code not found in match_info: this is a bug, please report it at \
                    https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&labels=bug&\
                    template=api_bug_report.yml&title=%5B500%5D%3A+code+not+found+in+match_info"
                        .to_string(),
                })
            }
        }
    };

    let user_id = auth.0;
    let bigint_user_id = u128_to_bigdecimal!(user_id);

    let db = get_db_or_fail!();

    let resp = sqlx::query!("SELECT * FROM invites WHERE code = $1", invite_code)
        .fetch_optional(db)
        .await;

    let guild_id = {
        match resp {
            Ok(ref resp) => match resp {
                Some(invite) => bigdecimal_to_u128!(invite.guild_id),
                None => {
                    return HttpResponse::NotFound().json(NotFoundJson {
                        message: "Invite not found.".to_string(),
                    })
                }
            },
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                })
            }
        }
    };

    let member_obj = match resp {
        Ok(resp) => match resp {
            Some(invite) => {
                let uses = invite.uses + 1;
                let unix_timestamp = OffsetDateTime::now_utc().unix_timestamp();
                let now = unix_timestamp - FERRIS_EPOCH;
                if let Some(max_uses) = invite.max_uses {
                    if uses > max_uses.into() {
                        let delete_resp =
                            sqlx::query!("DELETE FROM invites WHERE code = $1", invite_code)
                                .execute(db)
                                .await;

                        return match delete_resp {
                            Ok(_) => {
                                let invite_obj = Invite {
                                    code: invite.code.clone(),
                                    owner_id: bigdecimal_to_u128!(invite.owner_id),
                                    guild_id,
                                    created_at: invite.created_at,
                                    uses,
                                    max_uses: invite.max_uses,
                                    max_age: invite.max_age,
                                };

                                let event = WsOutboundEvent::InviteDelete { invite: invite_obj };

                                if let Err(e) =
                                    fire_event(format!("invite_{}", guild_id), &event).await
                                {
                                    let reason = match e {
                                        WsEventError::MissingRedis => {
                                            "Redis pool missing".to_string()
                                        }
                                        WsEventError::RedisError(e) => {
                                            format!("Redis returned an error: {}", e)
                                        }
                                        WsEventError::JsonError(e) => {
                                            format!(
                                                "Failed to serialize message to JSON format: {}",
                                                e
                                            )
                                        }
                                    };
                                    HttpResponse::InternalServerError()
                                        .json(InternalServerErrorJson { reason })
                                } else {
                                    HttpResponse::Gone().finish()
                                }
                            }
                            Err(e) => {
                                HttpResponse::InternalServerError().json(InternalServerErrorJson {
                                    reason: format!("DB returned an error: {}", e),
                                })
                            }
                        };
                    }
                };

                if let Some(max_age) = invite.max_age {
                    if (now - invite.created_at) > max_age {
                        let delete_resp =
                            sqlx::query!("DELETE FROM invites WHERE code = $1", invite_code)
                                .execute(db)
                                .await;

                        return match delete_resp {
                            Ok(_) => {
                                let invite_obj = Invite {
                                    code: invite.code.clone(),
                                    owner_id: bigdecimal_to_u128!(invite.owner_id),
                                    guild_id,
                                    created_at: invite.created_at,
                                    uses: invite.uses,
                                    max_uses: invite.max_uses,
                                    max_age: Some(max_age),
                                };

                                let event = WsOutboundEvent::InviteDelete { invite: invite_obj };

                                if let Err(e) =
                                    fire_event(format!("invite_{}", guild_id), &event).await
                                {
                                    let reason = match e {
                                        WsEventError::MissingRedis => {
                                            "Redis pool missing".to_string()
                                        }
                                        WsEventError::RedisError(e) => {
                                            format!("Redis returned an error: {}", e)
                                        }
                                        WsEventError::JsonError(e) => {
                                            format!(
                                                "Failed to serialize message to JSON format: {}",
                                                e
                                            )
                                        }
                                    };
                                    HttpResponse::InternalServerError()
                                        .json(InternalServerErrorJson { reason })
                                } else {
                                    HttpResponse::Gone().finish()
                                }
                            }
                            Err(e) => {
                                HttpResponse::InternalServerError().json(InternalServerErrorJson {
                                    reason: format!("DB returned an error: {}", e),
                                })
                            }
                        };
                    }
                }

                let already_exists = sqlx::query!(
                    r#"SELECT EXISTS(SELECT * FROM members WHERE user_id = $1 AND guild_id = $2) AS "exists!""#,
                    bigint_user_id,
                    invite.guild_id
                )
                .fetch_one(db)
                .await;
                match already_exists {
                    Ok(r) => {
                        if r.exists {
                            return HttpResponse::Conflict().json(InternalServerErrorJson {
                                reason: "user has already joined this guild".to_string(),
                            });
                        }
                    }
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("DB returned an error: {}", e),
                        })
                    }
                }

                let member_resp = sqlx::query!(
                    "INSERT INTO members VALUES ($1, $2)",
                    bigint_user_id,
                    invite.guild_id
                )
                .execute(db)
                .await;

                let member_obj = match member_resp {
                    Ok(_) => Member {
                        user_id: Some(user_id),
                        user: match sqlx::query!(
                            "SELECT * FROM users WHERE id = $1",
                            bigint_user_id
                        )
                        .fetch_optional(db)
                        .await
                        {
                            Ok(o) => match o {
                                Some(u) => Some(User {
                                    id: user_id,
                                    name: u.name.clone(),
                                    avatar: None,
                                    guilds: None,
                                    flags: UserFlags::from_bits_truncate(u.flags),
                                    discriminator: u.discriminator,
                                }),
                                None => None,
                            },
                            Err(e) => {
                                return HttpResponse::InternalServerError().json(
                                    InternalServerErrorJson {
                                        reason: format!("DB returned an error: {}", e),
                                    },
                                )
                            }
                        },
                        guild_id: Some(guild_id),
                        guild: None,
                    },
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!("DB returned an error: {}", e),
                        })
                    }
                };

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

                member_obj
            }
            None => return HttpResponse::NotFound().finish(),
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
            })
        }
    };

    let event = WsOutboundEvent::MemberCreate { member: member_obj };

    if let Err(e) = fire_event(format!("member_{}", guild_id), &event).await {
        let reason = match e {
            WsEventError::MissingRedis => "Redis pool missing".to_string(),
            WsEventError::RedisError(e) => format!("Redis returned an error: {}", e),
            WsEventError::JsonError(e) => {
                format!("Failed to serialize message to JSON format: {}", e)
            }
        };
        return HttpResponse::InternalServerError().json(InternalServerErrorJson { reason });
    }

    HttpResponse::Created().finish()
}
