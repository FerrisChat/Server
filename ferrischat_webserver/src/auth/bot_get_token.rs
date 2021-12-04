use crate::auth::token_gen::generate_random_bits;
use crate::{Json, WebServerError};
use axum::extract::Path;
use ferrischat_common::types::{AuthResponse, ErrorJson};
use futures::TryFutureExt;
use serde::Serialize;
use tokio::sync::oneshot::channel;

pub async fn get_bot_token(
    auth: crate::Authorization,
    Path(bot_id): Path<u128>,
) -> Result<Json<AuthResponse>, WebServerError> {
    let db = get_db_or_fail!();
    let bigint_bot_id = u128_to_bigdecimal!(bot_id);

    let bot_resp = sqlx::query!("SELECT * FROM bots WHERE user_id = $1", bigint_bot_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| {
            (
                404,
                ErrorJson::new_404(
                    format!("Unknown bot where ID = {}", bot_id)
                ),
            )
        })?;

    let owner_id = bigdecimal_to_u128!(bot_resp.owner_id);

    if owner_id != auth.0 {
        Ok(crate::Json {
            obj: ErrorJson::new_403(
                "Forbidden".to_string(),
            ),
            code: 403,
        })
    }

    let token = generate_random_bits()
        .map(|b| base64::encode_config(b, base64::URL_SAFE))
        .ok_or(WebServerError::RandomGenerationFailure)?;

    let (tx, rx) = channel();
    ferrischat_auth::GLOBAL_HASHER
        .get()
        .ok_or(WebServerError::MissingHasher)?
        .send((token.clone(), tx))
        .await
        .map_err(|_| {
            (
                500,
                ErrorJson::new_500(
                    "Password hasher has hung up connection".to_string(),
                    false,
                    None,
                ),
            )
                .into()
        })?;

    let hashed_token = rx.await
        .unwrap_or_else(|e| {
            unreachable!(
                "failed to receive value from channel despite value being sent earlier on: {}",
                e
            )
        })
        .map_err(|e| {
            (
                500,
                ErrorJson::new_500(
                    format!("failed to hash token: {}", e),
                    true,
                    Some(
                        "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                                         labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+failed+to+hash+token"
                            .to_string(),
                    ),
                ),
            )
                .into()
        })?;

    sqlx::query!(
        "INSERT INTO auth_tokens VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET auth_token = $2",
        bigint_bot_id,
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
