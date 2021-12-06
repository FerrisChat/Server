use crate::auth::token_gen::generate_random_bits;
use crate::WebServerError;
use axum::extract::Json;
use ferrischat_common::request_json::AuthJson;
use ferrischat_common::types::{AuthResponse, ErrorJson};
use sqlx::types::BigDecimal;

pub async fn get_token(
    Json(AuthJson { email, password }): Json<AuthJson>,
) -> Result<crate::Json<AuthResponse>, WebServerError> {
    let db = get_db_or_fail!();

    let r = sqlx::query!(
        "SELECT email, password, id FROM users WHERE email = $1",
        email
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| ErrorJson::new_404(format!("Unknown user with email {}", email)))?;
    let bigdecimal_user_id: BigDecimal = r.id;
    let matches = ferrischat_auth::verify(password, r.password).await?;
    if !(matches && (email == r.email)) {
        return Err(ErrorJson::new_404("Your credentials are not correct".to_string()).into());
    }

    let token = generate_random_bits()
        .map(|b| base64::encode_config(b, base64::URL_SAFE))
        .ok_or(WebServerError::RandomGenerationFailure)?;

    let hashed_token = ferrischat_auth::hash(&token).await?;

    sqlx::query!(
        "INSERT INTO auth_tokens VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET auth_token = $2",
        bigdecimal_user_id,
        hashed_token)
        .execute(db)
        .await?;

    let user_id = bigdecimal_to_u128!(bigdecimal_user_id);
    Ok(crate::Json {
        obj: AuthResponse {
            token: format!(
                "{}.{}",
                base64::encode_config(user_id.to_string(), base64::URL_SAFE),
                token,
            ),
        },
        code: 200,
    })
}
