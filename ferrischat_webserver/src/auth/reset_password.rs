use crate::auth::generate_random_bits;
use crate::WebServerError;
use axum::extract::Json;
use axum::extract::Path;
use ferrischat_common::request_json::PasswordResetJson;
use ferrischat_common::types::{ErrorJson, SuccessJson};
use ferrischat_redis::{redis::AsyncCommands, REDIS_MANAGER};
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};

/// POST /v0/auth/reset/{user_id}
/// Requires a new password encoded in JSON, like:
/// ```json
/// {
///   "password": "ASecurePassword"
/// }
/// ```
pub async fn reset_password(
    Path(user_id): Path<u128>,
    Json(PasswordResetJson { password }): Json<PasswordResetJson>,
) -> Result<crate::Json<SuccessJson>, WebServerError> {
    let db = get_db_or_fail!();
    let user = sqlx::query!(
        "SELECT verified, id, email FROM users WHERE id = $1",
        u128_to_bigdecimal!(user_id)
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ErrorJson::new_404("user does not exist".to_string()))?;

    if !user.verified {
        return Err(ErrorJson::new_403(
            "User must have a verified email to reset password!".to_string(),
        )
        .into());
    }

    // Get configurations, they're set in redis for speed reasons. Set them with redis-cli `set config:email:<setting> <value>`
    let mut redis = REDIS_MANAGER
        .get()
        .ok_or(WebServerError::MissingRedis)?
        .get()
        .await?;

    let smtp_host = redis
        // FQDN of the SMTP server
        .get::<&str, Option<String>>("config:email:host")
        .await?
        .ok_or_else(|| {
            ErrorJson::new_500("redis config not set (host)".to_string(), false, None)
        })?;
    let smtp_username = redis
        // FULL SMTP username, e.g. `system@ferris.chat`
        .get::<&str, Option<String>>("config:email:username")
        .await?
        .ok_or_else(|| {
            ErrorJson::new_500("redis config not set (username)".to_string(), false, None)
        })?;
    let smtp_password = redis
        // SMTP password
        .get::<&str, Option<String>>("config:email:password")
        .await?
        .ok_or_else(|| {
            ErrorJson::new_500("redis config not set (password)".to_string(), false, None)
        })?;
    let mail_creds = Credentials::new(smtp_username.clone(), smtp_password);
    let hashed_password = ferrischat_auth::hash(&password).await?;
    // This generates a random string that can be used to verify that the request is actually from the email owner
    let token = generate_random_bits()
        .map(|b| base64::encode_config(b, base64::URL_SAFE))
        .ok_or(WebServerError::RandomGenerationFailure)?;

    // Default email.
    // TODO HTML rather then plaintext
    // Also encodes the email to be URL-safe, however some work is needed on it still
    let default_email = format!(
        "Hey!\n\n\
        We see you have requested to reset your password. Click here to do so: https://api.ferris.chat/v0/auth/reset/{}.\n\n\
        If you did not request this, you can safely ignore it.\n\n\
        - FerrisChat Team\n\
        hello@ferris.chat",
        urlencoding::encode(&*token)
    );

    // Builds the message with a hardcoded subject and sender full name
    let message = Message::builder()
        .from(format!("FerrisChat System <{}>", smtp_username).parse()?)
        .to(user.email.parse()?)
        .subject("FerrisChat Password Reset")
        .body(default_email)?;

    // Open a remote, asynchronous connection to the mail server
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host.as_str())?
        .credentials(mail_creds)
        .build();

    // Send the email
    mailer.send(message).await?;

    let user_properties = format!("{}||||{}", user.id, hashed_password);

    // writes the token to redis.
    // The reason we use the token as the key rather then the value is so we can check against it more easily later, when it's part of the URL.
    redis
        .set_ex::<String, String, String>(
            format!("password-reset:{}", token),
            user_properties,
            86400,
        )
        .await?;

    Ok(crate::Json::new(
        SuccessJson::new("Sent password reset, please check your email.".to_string()),
        200,
    ))
}

/// GET /v0/auth/reset/{token}
/// Verifies the user's email when they click the link mailed to them.
pub async fn verify_password_reset(
    Path(token): Path<String>,
) -> Result<crate::Json<SuccessJson>, WebServerError> {
    let db = get_db_or_fail!();

    let redis_key = format!("password-reset:{}", token);
    let id_and_hashed_password_as_string = ferrischat_redis::redis::cmd("GETDEL")
        .arg(redis_key)
        .query_async::<_, Option<String>>(
            &mut REDIS_MANAGER
                .get()
                .ok_or(WebServerError::MissingRedis)?
                .get()
                .await?,
        )
        .await?
        .ok_or_else(|| {
            ErrorJson::new_404("This token has expired or was not found.".to_string())
        })?;
    let id_and_hashed_password_as_vec: Vec<&str> =
        id_and_hashed_password_as_string.split("||||").collect();
    // Tell the database to set their verified field to true! The user is now verified.
    sqlx::query!(
        "UPDATE users SET password = $1 WHERE id = $2",
        id_and_hashed_password_as_vec[0],
        u128_to_bigdecimal!(id_and_hashed_password_as_vec[1]
            .to_string()
            .parse::<sqlx::types::BigDecimal>()
            .map_err(|e| ErrorJson::new_500(format!("failed to parse user ID: {}", e), false, None)))?),
    )
    .execute(db)
    .await?;
    Ok(crate::Json::new(
        SuccessJson::new("Changed password. You can close this page.".to_string()),
        200,
    ))
}
