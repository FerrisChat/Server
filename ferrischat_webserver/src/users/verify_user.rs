use actix_web::{web::Json, HttpRequest, HttpResponse, Responder};
use check_if_email_exists::{check_email, CheckEmailInput, Reachable};
use ferrischat_common::types::{
    Guild, GuildFlags, InternalServerErrorJson, NotFoundJson, User, UserFlags,
};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use simd_json::ValueAccess;
use sqlx::Error;

use ferrischat_redis::{redis::AsyncCommands, REDIS_MANAGER};
/// POST /v0/verify
pub async fn send_verification_email(
    req: HttpRequest,
    auth: crate::Authorization,
) -> impl Responder {
    let db = get_db_or_fail!();
    let authorized_user = auth.0;
    let user_email = match sqlx::query!("SELECT email FROM users WHERE id = $1", u128_to_bigdecimal!(authorized_user))
        .fetch_one(db)
        .await
    {
        Ok(email) => email.email,
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("Database returned a error: {}", e),
            })
        }
    };
    let checker_input = CheckEmailInput::new(vec![user_email.into()]);
    let checked_email = check_email(&checker_input).await;
    if checked_email[0].syntax.is_valid_syntax {
        if checked_email[0].is_reachable == Reachable::Safe
            || checked_email[0].is_reachable == Reachable::Risky
            || checked_email[0].is_reachable == Reachable::Unknown
        {
            // get configs
            let mut redis = REDIS_MANAGER
                .get()
                .expect("redis not initialized: call load_redis before this")
                .clone();
            match host = redis
                .get::<String, String>("config:email:host".to_string())
                .await
            {
                Ok(_) => password,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("No SMTP server host set."),
                }),
            }
            match host = redis
                .get::<String, String>("config:email:username".to_string())
                .await
            {
                Ok(_) => password,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("No SMTP server username set."),
                }),
            }
            match password = redis
                .get::<String, String>("config:email:password".to_string())
                .await
            {
                Ok(_) => password,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("No SMTP server password set."),
                }),
            }
            let token = match crate::auth::generate_random_bits() {
                Some(b) => base64::encode_config(b, base64::URL_SAFE),
                None => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "failed to generate random bits for token generation".to_string(),
                    });
                }
            };
            match message = Message::builder().from("Ferris <verification@ferris.chat>".parse().unwrap()).reply_to("Ferris <hello@ferris.chat>".parse().unwrap()).to(user_email.parse().unwrap()).subject("FerrisChat Email Verification").body(String::from(format!("Welcome to FerrisChat!<br><a href=\"https://api.ferris.chat/v0/verify/{}\">Click here to verify \
        your email!</a> (expires in 1 hour) <br><br> If you don't know what this is, reset your token and change \
        your password ASAP.", token))) { Ok(_) => message, Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson { reason: format!("This should not have happened. Submit a bug report on \
                    https://github.com/ferrischat/server/issues with the error `{}`", e), }), }

            let mail_creds = Credentials::new(username.to_string(), password.to_string());

            // Open a remote connection to the mailserver
            match mailer = SmtpTransport::relay(host.as_string())
                .credentials(mail_creds)
                .build()
            {
                Ok(_) => mailer,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!(
                        "Error creating SMTP transport! Please submit a bug report on \
                    https://github.com/ferrischat/server/issues with the error `{}`",
                        e
                    ),
                }),
            }

            // Send the email
            if let Err(e) = mailer.send(&message) {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!(
                        "Mailer failed to send correctly! Please submit a bug report \
                    on https://github.com/ferrischat/server/issues with the error `{}`",
                        e
                    ),
                });
            }
            // writes the token to redis
            let r = redis
                .set_ex::<String, String, String>(
                    format!("email:tokens:{}", token),
                    user_email,
                    86400,
                )
                .await;
            HttpResponse::Ok().finish()
        } else {
            HttpResponse::Conflict().json(InternalServerErrorJson {
                reason: format!("Email deemed unsafe to send to. Is it a real email?"),
            })
        }
    } else {
        HttpResponse::Conflict().json(InternalServerErrorJson {
            reason: format!("Email {} is invalid.", user_email),
        })
    }
}
/// GET /v0/verify/{token}
pub async fn verify_email(req: HttpRequest, path: actix_web::web::Path<String>) -> impl Responder {
    let token = path.into_inner();
    let redis_key = format!("email:tokens:{}", token);
    
    let mut redis = REDIS_MANAGER
        .get()
        .expect("redis not initialized: call load_redis before this")
        .clone();
    match redis
        .get::<String, Option<String>>(redis_key)
        .await
    {
        Ok(Some(_)) => {
            if let Err(e) = redis.del::<String, i32>(redis_key).await {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("Redis returned a error: {}", e),
                });
            }
        }
        Ok(None) => {
            return HttpResponse::NotFound().json(NotFoundJson {
                message: format!("This token has expired or was not found."),
            });
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("Redis returned a error: {}", e),
            });
        }
    }
    if let Err(e) = sqlx::query!(
        "UPDATE users SET verified = true WHERE email = $1",
        authorized_user
    )
    .execute(db)
    .await
    {
        HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("Database returned a error: {}", e),
        })
    } else {
        HttpResponse::Ok().body("Verified email. You can close this page.")
    }
}
