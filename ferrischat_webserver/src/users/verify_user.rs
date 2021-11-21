use actix_web::{web, HttpResponse, Responder};
use check_if_email_exists::{check_email, CheckEmailInput, Reachable};
use ferrischat_common::types::{InternalServerErrorJson, Json, NotFoundJson};
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};

use ferrischat_redis::{redis::AsyncCommands, REDIS_MANAGER};
use tokio::time::Duration;

/// POST /v0/verify
/// Requires only an authorization token.
pub async fn send_verification_email(auth: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let authorized_user = auth.0;
    // Get the authorized user's email.
    let user_email = match sqlx::query!(
        "SELECT email FROM users WHERE id = $1",
        u128_to_bigdecimal!(authorized_user)
    )
    .fetch_one(db)
    .await
    {
        Ok(email) => email.email,
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("Database returned a error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };
    // Makes a call to the email checker to avoid sending to completely fake emails
    // TODO check if the user is verified already and send 304 Not Modified if they are
    let mut checker_input = CheckEmailInput::new(vec![user_email.clone().into()]);
    checker_input.set_smtp_timeout(Duration::new(5, 0));
    let checked_email = check_email(&checker_input).await;
    if checked_email[0].syntax.is_valid_syntax {
        if checked_email[0].is_reachable == Reachable::Safe
            || checked_email[0].is_reachable == Reachable::Risky
            || checked_email[0].is_reachable == Reachable::Unknown
        {
            // Get configurations, they're set in redis for speed reasons. Set them with redis-cli `set config:email:<setting> <value>`
            let mut redis = REDIS_MANAGER
                .get()
                .expect("redis not initialized: call load_redis before this")
                .clone();
            let host = match redis
                // FQDN of the SMTP server
                .get::<String, String>("config:email:host".to_string())
                .await
            {
                Ok(r) => r,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("No SMTP server host set."),
                        is_bug: false,
                        link: None,
                    })
                }
            };
            let username = match redis
                // FULL SMTP username, e.g. `verification@ferris.chat`
                .get::<String, String>("config:email:username".to_string())
                .await
            {
                Ok(r) => r,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("No SMTP server username set."),
                        is_bug: false,
                        link: None,
                    })
                }
            };
            let password = match redis
                // SMTP password
                .get::<String, String>("config:email:password".to_string())
                .await
            {
                Ok(r) => r,
                Err(_) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("No SMTP server password set."),
                        is_bug: false,
                        link: None,
                    })
                }
            };
            // This generates a random string that can be used to verify that the request is actually from the email owner
            let token = match crate::auth::generate_random_bits() {
                Some(b) => base64::encode_config(b, base64::URL_SAFE),
                None => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "failed to generate random bits for token generation".to_string(),
                        is_bug: true,
                        link: Option::from(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                                .to_string(),
                        ),
                    })
                }
            };
            // Default email.
            // TODO HTML rather then plaintext
            // Also encodes the email to be URL-safe, however some work is needed on it still
            let default_email = format!(
                "Click here to verify your email: https://api.ferris.chat/v0/verify/{}",
                urlencoding::encode(token.as_str()).into_owned()
            );
            // Builds the message with a hardcoded subject and sender full name
            let message = match Message::builder()
                .from(format!("Ferris <{}>", username).parse().unwrap())
                .to(user_email.parse().unwrap())
                .subject("FerrisChat Email Verification")
                .body(String::from(default_email))
            {
                Ok(m) => m,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!(
                        "This should not have happened. Submit a bug report with the error `{}`",
                        e
                    ),
                        is_bug: true,
                        link: Option::from(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                                .to_string(),
                        ),
                    })
                }
            };
            // simply gets credentials for the SMTP server
            let mail_creds = Credentials::new(username.to_string(), password.to_string());

            // Open a remote, asynchronous connection to the mailserver
            let mailer: AsyncSmtpTransport<Tokio1Executor> =
                match AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host.as_str()) {
                    Ok(m) => m.credentials(mail_creds).build(),
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: format!(
                                "Error creating SMTP transport! Please submit a bug report with the error `{}`",
                                e
                            ),
                            is_bug: true,
                            link: Option::from(
                                "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                                    .to_string()),
                        })
                    }
                };

            // Send the email
            if let Err(e) = mailer.send(message).await {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!(
                        "Mailer failed to send correctly! Please submit a bug report \
                    with the error `{}`",
                        e
                    ),
                    is_bug: true,
                    link: Option::from(
                        "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                            .to_string(),
                    ),
                });
            }
            // writes the token to redis.
            // The reason we use the token as the key rather then the value is so we can check against it more easily later, when it's part of the URL.
            if let Err(e) = redis
                .set_ex::<String, String, String>(
                    format!("email:tokens:{}", token),
                    user_email,
                    86400,
                )
                .await
            {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("Redis returned a error: {}", e),
                    is_bug: false,
                    link: None,
                });
            }
            HttpResponse::Ok().json(Json {
                message: format!("Sent verification, please check your email."),
            })
        } else {
            HttpResponse::Conflict().json(Json {
                message: format!("Email deemed unsafe to send to. Is it a real email?"),
            })
        }
    } else {
        HttpResponse::Conflict().json(Json {
            message: format!("Email {} is invalid.", user_email),
        })
    }
}
/// GET /v0/verify/{token}
/// Verifies the user's email when they click the linke mailed to them.
pub async fn verify_email(path: web::Path<String>) -> impl Responder {
    // Gets the last component of the path (should be the token) and searches redis for it
    let token = path.into_inner();
    let redis_key = format!("email:tokens:{}", token);

    let mut redis = REDIS_MANAGER
        .get()
        .expect("redis not initialized: call load_redis before this")
        .clone();
    // r/askredis
    let email = match redis.get::<String, Option<String>>(redis_key.clone()).await {
        Ok(Some(email)) => {
            if let Err(e) = redis.del::<String, i32>(redis_key).await {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("Redis returned a error: {}", e),
                    is_bug: false,
                    link: None,
                });
            }
            email
        }
        Ok(None) => {
            // TODO use 498 rather then 404
            return HttpResponse::NotFound().json(NotFoundJson {
                message: format!("This token has expired or was not found."),
            });
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("Redis returned a error: {}", e),
                is_bug: false,
                link: None,
            });
        }
    };
    let db = get_db_or_fail!();
    // Tell the database to set their verified field to true! The user is now verified.
    if let Err(e) = sqlx::query!("UPDATE users SET verified = true WHERE email = $1", email)
        .execute(db)
        .await
    {
        HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("Database returned a error: {}", e),
            is_bug: false,
            link: None,
        })
    } else {
        HttpResponse::Ok().body("Verified email. You can close this page.")
    }
}
