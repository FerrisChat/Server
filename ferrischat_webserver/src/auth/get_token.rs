use crate::auth::token_gen::generate_random_bits;
use actix_web::web::{self, HttpResponse, Path};
use actix_web::{HttpRequest, Responder};
use ferrischat_common::types::InternalServerErrorJson;
use ferrischat_macros::{get_db_or_fail, get_item_id};
use num_traits::FromPrimitive;
use sqlx::types::BigDecimal;
use tokio::sync::oneshot::channel;

pub async fn get_token(req: HttpRequest) -> impl Responder {
    let token = match generate_random_bits() {
        Some(b) => base64::encode_config(b, base64::URL_SAFE),
        None => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: "failed to generate random bits for token generation".to_string(),
            })
        }
    };
    let user_id = get_item_id!(req, "user_id");

    let hashed_token = {
        let rx = match crate::GLOBAL_HASHER.get() {
            Some(h) => {
                let (tx, rx) = channel();
                if let Err(e) = h.send((token.clone(), tx)).await {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "Password hasher has hung up connection".to_string(),
                    });
                };
                rx
            }
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "password hasher not found".to_string(),
                })
            }
        };
        match rx.await {
            Ok(r) => match r {
                Ok(r) => r,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("failed to hash token: {}", e),
                    })
                }
            },
            Err(e) => unreachable!(
                "failed to receive value from channel despite value being sent earlier on: {}",
                e
            ),
        }
    };

    let db = get_db_or_fail!();
    if let Err(e) = sqlx::query!("INSERT INTO auth_tokens VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET auth_token = $2", BigDecimal::from_u128(user_id), hashed_token).execute(db).await {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e)
        })
    };

    return HttpResponse::Ok().body(format!(
        "{}.{}",
        base64::encode_config(user_id.to_string(), base64::URL_SAFE),
        token,
    ));
}
