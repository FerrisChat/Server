use crate::auth::token_gen::generate_random_bits;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{
    AuthResponse, BadRequestJson, BadRequestJsonLocation, InternalServerErrorJson, NotFoundJson,
};
use tokio::sync::oneshot::channel;

pub async fn get_bot_token(auth: crate::Authorization, req: HttpRequest) -> impl Responder {
    let bigint_user_id = get_item_id!(req, "bot_id");
    let user_id = u128_to_bigdecimal!(bigint_user_id);
    let db = get_db_or_fail!();
    let owner_id_resp = match sqlx::query!(
        "SELECT * FROM bots WHERE user_id = $1",
        user_id,
    )
        .fetch_one(db)
        .await
    {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e)
        })
    };

    let u128_owner_id = bigdecimal_to_u128!(owner_id_resp.owner_id);

    if u128_owner_id != auth.0 {
        return HttpResponse::Forbidden().finish()
    }

    let token = match generate_random_bits() {
        Some(b) => base64::encode_config(b, base64::URL_SAFE),
        None => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: "failed to generate random bits for token generation".to_string(),
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

    if let Err(e) = sqlx::query!("INSERT INTO auth_tokens VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET auth_token = $2", user_id, hashed_token).execute(db).await {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e)
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
