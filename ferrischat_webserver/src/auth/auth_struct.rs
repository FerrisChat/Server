use crate::WebServerError;
use axum::body::Body;
use axum::extract::{FromRequest, RequestParts};
use ferrischat_common::types::ErrorJson;
use tokio::sync::oneshot::channel;

pub struct Authorization(pub u128);

#[async_trait::async_trait]
impl FromRequest for Authorization {
    type Rejection = crate::WebServerError;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let headers = req.headers().ok_or_else(|| ErrorJson::new_500(
            "another extractor took headers".to_string(),
            true,
            Some(
                "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+another+extractor+took+headers"
                    .to_string(),
            ),
        ))?;

        let auth_header = headers
            .get(http::header::AUTHORIZATION)
            .ok_or_else(|| ErrorJson::new_400("Authorization header not found".to_string()))?;

        let token = String::from_utf8(auth_header.as_bytes().into()).map_err(|e| {
            ErrorJson::new_400(format!(
                "Authorization header contained invalid UTF-8: {}",
                e
            ))
        })?;

        let mut auth = token.split('.');

        let str_id = auth
            .next()
            .ok_or_else(|| ErrorJson::new_400("Authorization header was empty".to_string()))?;

        let bin_id = base64::decode_config(str_id, base64::URL_SAFE).map_err(|e| {
            ErrorJson::new_400(format!(
                "Authorization header contained invalid base64: {}",
                e
            ))
        })?;

        let str_id = String::from_utf8(bin_id).map_err(|e| {
            ErrorJson::new_400(format!(
                "Authorization header contained invalid UTF-8: {}",
                e
            ))
        })?;

        let id = str_id.parse::<u128>().map_err(|e| {
            ErrorJson::new_400(format!(
                "Authorization header contained invalid integer: {}",
                e
            ))
        })?;

        let id_bigint = u128_to_bigdecimal!(id);

        let db = get_db_or_fail!();

        let db_token = sqlx::query!(
            "SELECT (auth_token) FROM auth_tokens WHERE user_id = $1",
            id_bigint
        )
        .fetch_optional(db)
        .await?
        .map(|t| t.auth_token)
        .ok_or_else(|| ErrorJson::new_401("Authorization header passed was invalid".to_string()))?;

        let verifier = ferrischat_auth::GLOBAL_VERIFIER
            .get()
            .ok_or(WebServerError::MissingVerifier)?;
        let (tx, rx) = channel();

        // if the send failed, we'll know because the receiver we wait upon below will fail instantly
        let _tx = verifier.send(((token, db_token), tx)).await;
        let valid = rx.await.map_err(|_| ErrorJson::new_500("Global auth verifier not initialized or missing".to_string(), true, Some(
            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+global+hash+verifier+not+found"
                .to_string(),
        )))?.map_err(|e| ErrorJson::new_500(format!("Failed to verify token: {}", e), false, None))?;
        if valid {
            Ok(Self(id))
        } else {
            // we specifically do not define the boundary between no token and
            // wrong tokens
            Err(ErrorJson::new_401("Authorization header passed was invalid".to_string()).into())
        }
    }
}
