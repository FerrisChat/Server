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
    let authorized_user = auth.0;
    /// GET /v0/verify/{token}
    pub async fn verify_email(req: HttpRequest, token: actix_web::web::Path<String>) -> impl Responder {
        let mut redis = REDIS_MANAGER
            .get()
            .expect("redis not initialized: call load_redis before this")
            .clone();
        match host = redis
            .get(format!("email:tokens:{}", token.0))
            .get::<String, String>()
            .await
        {
            Ok(_) => password,
            Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("No token for you found."),
            }),
        }
    }
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
            match host = redis.get("config:email:host").get::<String, String>().await {
                Ok(_) => password,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("No SMTP server host set."),
                }),
            }
            match host = redis
                .get("config:email:username")
                .get::<String, String>()
                .await
            {
                Ok(_) => password,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("No SMTP server username set."),
                }),
            }
            match password = redis
                .get("config:email:password")
                .get::<String, String>()
                .await
            {
                Ok(_) => password,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("No SMTP server password set."),
                }),
            }
            let token = match ferrischat_webserver::auth::generate_random_bits() {
                Some(b) => base64::encode_config(b, base64::URL_SAFE),
                None => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "failed to generate random bits for token generation".to_string(),
                    })
                }
            };
            match message = Message::builder()
                .from("Ferris <verification@ferris.chat>".parse().unwrap())
                .reply_to("Ferris <hello@ferris.chat>".parse().unwrap())
                .to(user_email.parse().unwrap())
                .subject("FerrisChat Email Verification")
                .body(String::from(format!(
                    "Welcome to FerrisChat!<br><a href=\"https://api.ferris.chat/v0/verify/{}\">Click here to verify \
        your email!</a> (expires in 1 hour) <br><br> If you don't know what this is, reset your token and change \
        your password ASAP.",
                    token
                ))) {
                Ok(_) => message,
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!(
                        "This should not have happened. Submit a bug report on \
                    https://github.com/ferrischat/server/issues with the error `{}`",
                        e
                    ),
                }),
            }

            let mail_creds = Credentials::new(username.to_string(), password.to_string());

            // Open a remote connection to gmail
            match mailer = SmtpTransport::relay(host.as_str().unwrap())
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
            match mailer.send(&message) {
                Ok(_) => HttpResponse::Ok().finish(),
                Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!(
                        "Mailer failed to send correctly! Please submit a bug report \
                    on https://github.com/ferrischat/server/issues with the error `{}`",
                        e
                    ),
                }),
            }
            // writes the token to redis
            let r = redis
                .set_ex::<String, String, String>(
                    format!("email:tokens:{}", token),
                    user_email,
                    86400,
                )
                .await;
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
    let mut redis = REDIS_MANAGER
        .get()
        .expect("redis not initialized: call load_redis before this")
        .clone();
    match host = redis
        .get(format!("email:tokens:{}", path.into_inner()))
        .get::<String, String>()
        .await
    {
        Ok(_) => HttpResponse::Ok().body("Verified email. You can close this page."),
        Err(e) => HttpResponse::NotFound().json(NotFoundJson {
            message: format!("This token has expired or was not found.")
        }),
    }
}
