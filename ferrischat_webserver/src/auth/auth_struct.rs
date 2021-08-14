use actix_web::dev::Payload;
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized};
use actix_web::http::HeaderValue;
use actix_web::Error;
use actix_web::{FromRequest, HttpRequest};
use futures::Future;
use num_traits::FromPrimitive;
use sqlx::types::BigDecimal;
use tokio::sync::oneshot::channel;

macro_rules! parse_b64_to_string {
    ($input:expr) => {{
        match base64::decode_config($input, base64::URL_SAFE) {
            Ok(t) => match String::from_utf8(t) {
                Ok(s) => s,
                Err(e) => {
                    return Err(ErrorBadRequest(format!(
                        "`Authorization` header contained illegal UTF-8: {}",
                        e
                    )))
                }
            },
            Err(e) => {
                return Err(ErrorBadRequest(format!(
                    "`Authorization` header contained illegal base64: {}",
                    e
                )))
            }
        }
    }};
}

pub struct Authorization(pub u128);
impl FromRequest for Authorization {
    type Error = Error;
    type Future = impl Future<Output = Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone(); // we do not speak of this
        async move {
            let auth: HeaderValue = match req.headers().get("Authorization") {
                Some(h) => h.clone(),
                None => {
                    return Err(ErrorUnauthorized(
                        "No `Authorization` header passed".to_string(),
                    ));
                }
            };
            let token = match String::from_utf8(Vec::from(auth.as_ref())) {
                Ok(s) => s,
                Err(e) => {
                    return Err(ErrorBadRequest(format!(
                        "Authorization header contained illegal UTF-8: {}",
                        e
                    )));
                }
            };

            let mut auth = token.split(".");
            let id = match auth.next() {
                Some(id) => match parse_b64_to_string!(id).parse::<u128>() {
                    Ok(id) => id,
                    Err(e) => {
                        return Err(ErrorBadRequest(format!(
                            "`Authorization` header contained invalid integer: {}",
                            e
                        )));
                    }
                },
                None => {
                    return Err(ErrorBadRequest(
                        "`Authorization` header was empty".to_string(),
                    ));
                }
            };
            let token = match auth.next() {
                Some(token) => token.to_string(),
                None => {
                    return Err(ErrorBadRequest(
                        "`Authorization` header contained only one part".to_string(),
                    ));
                }
            };
            let id_bigint = BigDecimal::from_u128(id);
            let db = match ferrischat_db::DATABASE_POOL.get() {
                Some(db) => db,
                None => {
                    return Err(ErrorInternalServerError(
                        "Database pool was not initialized".to_string(),
                    ));
                }
            };

            let db_token = match sqlx::query!(
                "SELECT (auth_token) FROM auth_tokens WHERE user_id = $1",
                id_bigint
            )
            .fetch_optional(db)
            .await
            {
                Ok(t) => match t {
                    Some(t) => t.auth_token,
                    None => {
                        return Err(ErrorUnauthorized(
                            "`Authorization` header passed was invalid",
                        ));
                    }
                },
                Err(e) => {
                    return Err(ErrorInternalServerError(format!(
                        "Database returned a error: {}",
                        e
                    )));
                }
            };
            let verifier = match crate::GLOBAL_VERIFIER.get() {
                Some(v) => v,
                None => {
                    return Err(ErrorInternalServerError(
                        "Global hash verifier not found".to_string(),
                    ))
                }
            };
            let (tx, rx) = channel();
            // if the send failed, we'll know because the receiver we wait upon below will fail instantly
            let _ = verifier.send(((token, db_token), tx)).await;
            let res = match rx.await {
                Ok(r) => match r {
                    Ok(r) => r,
                    Err(e) => {
                        return Err(ErrorInternalServerError(format!(
                            "Failed to verify token: {}",
                            e
                        )))
                    }
                },
                Err(_) => {
                    return Err(ErrorInternalServerError(
                        "A impossible situation seems to have happened".to_string(),
                    ))
                }
            };
            if res {
                return Ok(Self(id));
            } else {
                // we specifically do not define the boundary between no token and
                // wrong tokens
                Err(ErrorUnauthorized(
                    "`Authorization` header passed was invalid",
                ))
            }
        }
    }
}
