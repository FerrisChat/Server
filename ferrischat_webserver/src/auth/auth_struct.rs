use axum::body::Body;
use axum::extract::{FromRequest, RequestParts};
use axum::http::HeaderMap;
use futures::Future;
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
#[async_trait]
impl FromRequest for Authorization {
    type Rejection = crate::WebServerError;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let headers = match req.headers() {
            Some(h) => h,
            None => {
                return Err((500, "another extractor took headers").into());
            }
        };

        /*
        {
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

                let mut auth = token.split('.');
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
                let id_bigint = u128_to_bigdecimal!(id);
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
                let verifier = match ferrischat_auth::GLOBAL_VERIFIER.get() {
                    Some(v) => v,
                    None => {
                        return Err(ErrorInternalServerError(
                            "Global hash verifier not found".to_string(),
                        ))
                    }
                };
                let (tx, rx) = channel();
                // if the send failed, we'll know because the receiver we wait upon below will fail instantly
                let _tx = verifier.send(((token, db_token), tx)).await;
                let valid = match rx.await {
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
                if valid {
                    Ok(Self(id))
                } else {
                    // we specifically do not define the boundary between no token and
                    // wrong tokens
                    Err(ErrorUnauthorized(
                        "`Authorization` header passed was invalid",
                    ))
                }
            }

         */
    }
}
