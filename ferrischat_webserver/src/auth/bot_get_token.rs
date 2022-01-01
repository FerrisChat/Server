use crate::auth::token_gen::generate_random_bits;
use crate::{Json, WebServerError};
use axum::extract::Path;
use ferrischat_common::types::{AuthResponse, ErrorJson};

pub async fn get_bot_token(
    auth: crate::Authorization,
    Path(bot_id): Path<u128>,
) -> Result<Json<AuthResponse>, WebServerError> {
    let db = get_db_or_fail!();
    let bigdecimal_bot_id = u128_to_bigdecimal!(bot_id);

    let bot_resp = sqlx::query!("SELECT * FROM bots WHERE user_id = $1", bigdecimal_bot_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown bot with ID {}", bot_id)))?;

    let owner_id = bigdecimal_to_u128!(bot_resp.owner_id);

    if owner_id != auth.0 {
        return Err(ErrorJson::new_403("you are not the owner of this bot".to_string()).into());
    }

    let token = generate_random_bits()
        .map(|b| base64::encode_config(b, base64::URL_SAFE))
        .ok_or(WebServerError::RandomGenerationFailure)?;

    let hashed_token = ferrischat_auth::hash(&token).await?;

    sqlx::query!(
        "INSERT INTO auth_tokens VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET auth_token = $2",
        bigdecimal_bot_id,
        hashed_token)
        .execute(db)
        .await?;

    Ok(Json {
        obj: AuthResponse {
            token: format!(
                "{}.{}",
                base64::encode_config(bot_id.to_string(), base64::URL_SAFE),
                token,
            ),
        },
        code: 200,
    })
}
