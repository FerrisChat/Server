use crate::auth::token_gen::generate_random_bits;
use crate::{Json, WebServerError};
use axum::extract::Path;
use ferrischat_common::types::{AuthResponse, InternalServerErrorJson};
use tokio::sync::oneshot::channel;

pub async fn get_bot_token(
    auth: crate::Authorization,
    Path(user_id): Path<u128>,
) -> Result<Json<T>, WebServerError<T>> {
    let bigint_user_id = u128_to_bigdecimal!(user_id);
    let db = get_db_or_fail!();
    let bigint_owner_id = sqlx::query!("SELECT * FROM bots WHERE user_id = $1", bigint_user_id)
        .fetch_one(db)
        .await?
        .owner_id;

    let owner_id = bigdecimal_to_u128!(bigint_owner_id);

    if owner_id != auth.0 {
        return HttpResponse::Forbidden().finish();
    }

    let token = match generate_random_bits() {
        Some(b) => base64::encode_config(b, base64::URL_SAFE),
        None => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: "failed to generate random bits for token generation".to_string(),
                is_bug: true,
                link: Some(
                    "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+failed+to+generate+random+bits+for+token+gen"
                        .to_string(),
                ),
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
                    is_bug: false,
                    link: None,
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
                        link: Some(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                                .to_string(),
                        ),
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
    return HttpResponse::Ok().json(AuthResponse {
        token: format!(
            "{}.{}",
            base64::encode_config(user_id.to_string(), base64::URL_SAFE),
            token,
        ),
    });
}
