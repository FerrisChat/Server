use crate::auth::generate_random_bits;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{ErrorJson, SuccessJson};
use ferrischat_redis::{redis::AsyncCommands, REDIS_MANAGER};
use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport, Message,
    Tokio1Executor,
};

/// POST /v0/verify
/// Requires only an authorization token.
pub async fn send_verification_email(
    crate::Authorization(authorized_user, is_bot): crate::Authorization,
) -> Result<crate::Json<SuccessJson>, WebServerError> {
    if is_bot {
        return Err(ErrorJson::new_403("Bots cannot be verified by email".to_string()).into());
    };
    let db = get_db_or_fail!();
    let bigdecimal_user_id = u128_to_bigdecimal!(authorized_user);

    // Get the authorized user's email.
    let user_email = sqlx::query!("SELECT email FROM users WHERE id = $1", bigdecimal_user_id)
        .fetch_one(db)
        .await?
        .email;

    if sqlx::query!(
        "SELECT verified FROM users WHERE id = $1",
        u128_to_bigdecimal!(authorized_user)
    )
    // you can safely assert that the user already exists because the authorization would've failed otherwise
    .fetch_one(db)
    .await?
    .verified
    {
        return Err(ErrorJson::new_409("User is already verified!".to_string()).into());
    }

    // Get configurations, they're set in redis for speed reasons. Set them with redis-cli `set config:email:<setting> <value>`
    let mut redis = REDIS_MANAGER
        .get()
        .ok_or(WebServerError::MissingRedis)?
        .get()
        .await?;

    let host = redis
        // FQDN of the SMTP server
        .get::<&str, Option<String>>("config:email:host")
        .await?
        .ok_or_else(|| {
            ErrorJson::new_500("redis config not set (host)".to_string(), false, None)
        })?;
    let username = redis
        // FULL SMTP username, e.g. `verification@ferris.chat`
        .get::<&str, Option<String>>("config:email:username")
        .await?
        .ok_or_else(|| {
            ErrorJson::new_500("redis config not set (username)".to_string(), false, None)
        })?;
    let password = redis
        // SMTP password
        .get::<&str, Option<String>>("config:email:password")
        .await?
        .ok_or_else(|| {
            ErrorJson::new_500("redis config not set (password)".to_string(), false, None)
        })?;
    let mail_creds = Credentials::new(username.clone(), password);

    // This generates a random string that can be used to verify that the request is actually from the email owner
    let token = generate_random_bits()
        .map(|b| base64::encode_config(b, base64::URL_SAFE))
        .ok_or(WebServerError::RandomGenerationFailure)?;

    // Default email.
    // TODO HTML rather then plaintext
    // Also encodes the email to be URL-safe, however some work is needed on it still
    let default_email = format!(
        "Hey!\n\nWe see you have requested to verify your email. Click here to verify your email: https://api.ferris.chat/v0/verify/{}.\n\nIf you did not request this, your account may be compromised.\n\n- FerrisChat Team\nhello@ferris.chat",
        urlencoding::encode(&*token)
    );

    // Builds the message with a hardcoded subject and sender full name
    let message = Message::builder()
        .from(format!("FerrisChat System <{}>", username).parse()?)
        .to(user_email.parse()?)
        .subject("FerrisChat Email Verification")
        .body(default_email)?;

    // Open a remote, asynchronous connection to the mail server
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host.as_str())?
        .credentials(mail_creds)
        .build();

    // Send the email
    mailer.send(message).await?;

    // writes the token to redis.
    // The reason we use the token as the key rather then the value is so we can check against it more easily later, when it's part of the URL.
    redis
        .set_ex::<String, String, String>(format!("email:tokens:{}", token), user_email, 86400)
        .await?;

    Ok(crate::Json::new(
        SuccessJson::new("Sent verification, please check your email.".to_string()),
        200,
    ))
}

/// GET /v0/verify/{token}
/// Verifies the user's email when they click the link mailed to them.
pub async fn verify_email(
    Path(token): Path<String>,
) -> Result<crate::Json<SuccessJson>, WebServerError> {
    let db = get_db_or_fail!();

    let redis_key = format!("email:tokens:{}", token);
    let email = ferrischat_redis::redis::cmd("GETDEL")
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

    // Tell the database to set their verified field to true! The user is now verified.
    sqlx::query!("UPDATE users SET verified = true WHERE email = $1", email)
        .execute(db)
        .await?;
    Ok(crate::Json::new(
        SuccessJson::new("Verified email. You can close this page.".to_string()),
        200,
    ))
}
