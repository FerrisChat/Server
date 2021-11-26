use axum::body::Body;
use axum::extract::{FromRequest, RequestParts};
use ferrischat_common::types::{BadRequestJson, InternalServerErrorJson, Json};
use serde::Serialize;
use std::marker::PhantomData;
use tokio::sync::oneshot::channel;

pub struct Authorization<T: Serialize>(pub u128, PhantomData<T>);

#[async_trait::async_trait]
impl<T: Serialize> FromRequest for Authorization<T> {
    type Rejection = crate::WebServerError<T>;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let headers = match req.headers() {
            Some(h) => h,
            None => {
                return Err((
                    500,
                    InternalServerErrorJson {
                        reason: "another extractor took headers".to_string(),
                        is_bug: true,
                        link: Some(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                            labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+another+extractor+took+headers"
                                .to_string(),
                        ),
                    },
                )
                    .into())
            }
        };
        let auth_header = match headers.get(http::header::AUTHORIZATION) {
            Some(v) => v,
            None => {
                return Err((
                    400,
                    BadRequestJson {
                        reason: "Authorization header not found".to_string(),
                        location: None,
                    },
                )
                    .into())
            }
        };
        let token = match String::from_utf8(auth_header.as_bytes().into()) {
            Ok(t) => t,
            Err(e) => {
                return Err((
                    400,
                    BadRequestJson {
                        reason: format!("Authorization header contained invalid UTF-8: {}", e),
                        location: None,
                    },
                )
                    .into())
            }
        };
        let mut auth = token.split('.');

        let str_id = match auth.next() {
            Some(s) => s,
            None => {
                return Err((
                    400,
                    BadRequestJson {
                        reason: "Authorization header was empty".to_string(),
                        location: None,
                    },
                )
                    .into());
            }
        };

        let bin_id = match base64::decode_config(str_id, base64::URL_SAFE) {
            Ok(id) => id,
            Err(e) => {
                return Err((
                    400,
                    BadRequestJson {
                        reason: format!("Authorization header contained invalid base64: {}", e),
                        location: None,
                    },
                )
                    .into())
            }
        };

        let str_id = match String::from_utf8(bin_id) {
            Ok(s) => s,
            Err(e) => {
                return Err((
                    400,
                    BadRequestJson {
                        reason: format!("Authorization header contained invalid UTF-8: {}", e),
                        location: None,
                    },
                )
                    .into())
            }
        };
        let id = match str_id.parse::<u128>() {
            Ok(id) => id,
            Err(e) => {
                return Err((
                    400,
                    BadRequestJson {
                        reason: format!("Authorization header contained invalid integer: {}", e),
                        location: None,
                    },
                )
                    .into())
            }
        };
        let id_bigint = u128_to_bigdecimal!(id);

        let db = match ferrischat_db::DATABASE_POOL.get() {
            Some(db) => db,
            None => {
                return Err((
                    500,
                    InternalServerErrorJson {
                        reason: "Database pool was not initialized".to_string(),
                    },
                )
                    .into())
            }
        };

        let db_token = match sqlx::query!(
            "SELECT (auth_token) FROM auth_tokens WHERE user_id = $1",
            id_bigint
        )
        .fetch_optional(db)
        .await
        {
            Ok(Some(t)) => t.auth_token,
            Ok(None) => {
                return Err((
                    401,
                    Json {
                        message: "Authorization header passed was invalid".to_string(),
                    },
                )
                    .into());
            }
            Err(e) => {
                return Err((
                    500,
                    InternalServerErrorJson {
                        reason: format!("Database returned an error: {}", e),
                        is_bug: false,
                        link: None,
                    },
                )
                    .into());
            }
        };

        let verifier =
            match ferrischat_auth::GLOBAL_VERIFIER.get() {
                Some(v) => v,
                None => return Err((
                    500,
                    InternalServerErrorJson {
                        reason: "Global hash verifier not found".to_string(),
                        is_bug: true,
                        link: Some(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                            labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+global+hash+verifier+not+found"
                                .to_string(),
                        ),
                    },
                )
                    .into()),
            };
        let (tx, rx) = channel();

        // if the send failed, we'll know because the receiver we wait upon below will fail instantly
        let _tx = verifier.send(((token, db_token), tx)).await;
        let valid = match rx.await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err((500, InternalServerErrorJson {
                reason: format!("Failed to verify token: {}", e),
                is_bug: false,
                reason: None,
            }).into()),
            Err(_) => {
                return Err((
                    500,
                    InternalServerErrorJson {
                        reason: "Global auth verifier not initialized or missing".to_string(),
                        is_bug: true,
                        link: Some(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                            labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+global+hash+verifier+not+found"
                                .to_string(),
                        ),
                    },
                )
                    .into())
            }
        };
        if valid {
            Ok(Self(id, PhantomData))
        } else {
            // we specifically do not define the boundary between no token and
            // wrong tokens
            Err((
                401,
                Json {
                    message: "Authorization header passed was invalid".to_string(),
                },
            )
                .into())
        }
    }
}
