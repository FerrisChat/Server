use crate::auth::token_gen::generate_random_bits;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{
    AuthResponse, BadRequestJson, BadRequestJsonLocation, InternalServerErrorJson, NotFoundJson,
};
use tokio::sync::oneshot::channel;

pub async fn get_token(req: HttpRequest) -> impl Responder {
    let headers = req.headers();
    let user_email = match headers.get("Email") {
        Some(e) => match String::from_utf8(Vec::from(e.as_bytes())) {
            Ok(e) => e,
            Err(e) => {
                return HttpResponse::BadRequest().json(BadRequestJson {
                    reason: "`Email` header contained invalid UTF-8".to_string(),
                    location: Some(BadRequestJsonLocation {
                        line: 0,
                        character: (e.utf8_error().valid_up_to() + 1) as u32,
                    }),
                })
            }
        },
        None => {
            return HttpResponse::BadRequest().json(BadRequestJson {
                reason: "No `Email` header passed".to_string(),
                location: None,
            })
        }
    };
    let user_password = match headers.get("Password") {
        Some(p) => match String::from_utf8(Vec::from(p.as_bytes())) {
            Ok(p) => p,
            Err(e) => {
                return HttpResponse::BadRequest().json(BadRequestJson {
                    reason: "`Password` header contained invalid UTF-8".to_string(),
                    location: Some(BadRequestJsonLocation {
                        line: 0,
                        character: (e.utf8_error().valid_up_to() + 1) as u32,
                    }),
                })
            }
        },
        None => {
            return HttpResponse::BadRequest().json(BadRequestJson {
                reason: "No `Password` header passed".to_string(),
                location: None,
            })
        }
    };

    let db = get_db_or_fail!();

    let bigint_user_id = match sqlx::query!(
        "SELECT email, password, id FROM users WHERE email = $1",
        user_email
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(mut r)) => {
            let matches = {
                let rx = match ferrischat_auth::GLOBAL_VERIFIER.get() {
                    Some(v) => {
                        let (tx, rx) = channel();
                        let db_password = std::mem::take(&mut r.password);
                        if v.send(((user_password, db_password), tx)).await.is_err() {
                            return HttpResponse::InternalServerError().json(
                                InternalServerErrorJson {
                                    reason: "Password verifier has hung up connection".to_string(),
                                    is_bug: false,
                                    link: None,
                                },
                            );
                        };
                        rx
                    }
                    None => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: "password verifier not found".to_string(),
                            is_bug: true,
                            link: Option::from(
                                "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                                    .to_string()),
                        })
                    }
                };
                match rx.await {
                    Ok(r) => {
                        match r {
                            Ok(d) => d,
                            Err(e) => return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                                reason: format!("failed to verify password: {}", e),
                                is_bug: true,
                                link: None,
                            })
                        }
                    }
                    Err(e) => unreachable!(
                        "failed to receive value from channel despite value being sent earlier on: {}",
                        e
                    )
                }
            };
            if !(matches && (user_email == r.email)) {
                return HttpResponse::Unauthorized().finish();
            }
            r.id
        }
        Ok(None) => {
            return HttpResponse::NotFound().json(NotFoundJson {
                message: format!("Unknown user with email {}", user_email),
            })
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned a error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };

    let token = match generate_random_bits() {
        Some(b) => base64::encode_config(b, base64::URL_SAFE),
        None => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: "failed to generate random bits for token generation".to_string(),
                is_bug: true,
                link: Option::from(
                    "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                        .to_string()),
            })
        }
    };

    let hashed_token = {
        let rx = match ferrischat_auth::GLOBAL_HASHER.get() {
            Some(h) => {
                let (tx, rx) = channel();
                if h.send((token.clone(), tx)).await.is_err() {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "Password hasher has hung up connection".to_string(),
                        is_bug: false,
                        link: None,
                    });
                };
                rx
            }
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "password hasher not found".to_string(),
                    is_bug: true,
                    link: Option::from(
                        "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                            .to_string()),
                })
            }
        };
        match rx.await {
            Ok(r) => match r {
                Ok(r) => r,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("failed to hash token: {}", e),
                        is_bug: true,
                        link: Option::from(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                                .to_string()),
                    })
                }
            },
            Err(e) => unreachable!(
                "failed to receive value from channel despite value being sent earlier on: {}",
                e
            ),
        }
    };

    if let Err(e) = sqlx::query!("INSERT INTO auth_tokens VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET auth_token = $2", bigint_user_id, hashed_token).execute(db).await {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
            is_bug: false,
            link: None,
        })
    };

    let user_id = bigdecimal_to_u128!(bigint_user_id);
    return HttpResponse::Ok().json(AuthResponse {
        token: format!(
            "{}.{}",
            base64::encode_config(user_id.to_string(), base64::URL_SAFE),
            token,
        ),
    });
}
