use crate::verify_token::{verify_token, VerifyTokenFailure};
use crate::SplitTokenError;
use actix_web::dev::Payload;
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorUnauthorized};
use actix_web::http::HeaderValue;
use actix_web::Error;
use actix_web::{FromRequest, HttpRequest};
use futures::Future;

pub struct Authorization(pub u128);
impl FromRequest for Authorization {
    type Config = ();
    type Error = Error;
    type Future = impl Future<Output = Result<Self, Self::Error>>;

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

            let (id, secret) = match crate::split_token(token) {
                Ok(t) => t,
                Err(e) => {
                    return Err(ErrorBadRequest(match e {
                        SplitTokenError::InvalidUtf8(e) => {
                            format!("invalid utf-8 detected while decoding token: {}", e)
                        }
                        SplitTokenError::Base64DecodeError(e) => {
                            format!("invalid base64 detected while decoding token: {}", e)
                        }
                        SplitTokenError::InvalidInteger(e) => format!(
                            "invalid integer detected while parsing user ID in token: {}",
                            e
                        ),
                        SplitTokenError::MissingParts(e) => format!("part {} of token missing", e),
                    }))
                }
            };

            verify_token(id, secret)
                .await
                .map(|_| Self(id))
                .map_err(|e| match e {
                    VerifyTokenFailure::InternalServerError(e) => ErrorInternalServerError(e),
                    VerifyTokenFailure::Unauthorized(e) => ErrorUnauthorized(e),
                })
        }
    }
}
