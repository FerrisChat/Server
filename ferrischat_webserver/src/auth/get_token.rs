use crate::auth::token_gen::generate_random_bits;
use crate::special_headers::{Email, Password};
use crate::{Json, WebServerError};
use axum::extract::TypedHeader;
use ferrischat_common::types::{AuthResponse, ErrorJson};
use serde::Serialize;
use sqlx::types::BigDecimal;
use tokio::sync::oneshot::channel;

pub async fn get_token(
    TypedHeader(email): TypedHeader<Email>,
    TypedHeader(password): TypedHeader<Password>,
) -> Result<Json<AuthResponse>, WebServerError> {
    let user_email = email.into_inner();
    let user_password = password.into_inner();

    let db = get_db_or_fail!();

    let mut r = sqlx::query!(
        "SELECT email, password, id FROM users WHERE email = $1",
        user_email
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        (
            404,
            ErrorJson::new_404(
                format!("Unknown user with email {}", user_email),
            ),
        )
            .into()
    })?;
    let bigdecimal_user_id: BigDecimal = r.id;
    let matches = {
        let v = ferrischat_auth::GLOBAL_VERIFIER
            .get()
            .ok_or(WebServerError::MissingVerifier)?;
        let (tx, rx) = channel();
        let db_password = std::mem::take(&mut r.password);
        v.send(((user_password, db_password), tx))
            .await
            .map_err(|_| ErrorJson::new_500(
                "Password verifier has hung up connection".to_string(),
                false,
                None,
            ))?;
        rx.await
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
                        format!("failed to verify password: {}", e),
                        false,
                        None,
                    ),
                )
                    .into()
            })?
    };
    if !(matches && (user_email == r.email)) {
        return Err((
            404,
            ErrorJson::new_404(
                format!("Your credentials are not correct"),
            ),
        )
            .into());
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
        bigdecimal_user_id,
        hashed_token)
        .execute(db)
        .await?;

    let user_id = bigdecimal_to_u128!(bigdecimal_user_id);
    Ok(Json {
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
