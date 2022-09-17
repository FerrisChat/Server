use crate::{Error, HeaderAwareError, Response, StatusCode};

use argon2_async::{set_config, Config};
use axum::{
    body::Body,
    extract::{FromRequest, RequestParts},
};
use base64::{encode_config, URL_SAFE_NO_PAD};
use ring::rand::{SecureRandom, SystemRandom};

use std::{
    fs,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

pub static RNG: OnceLock<SystemRandom> = OnceLock::new();
pub const TOKEN_EPOCH: u128 = common::util::FERRIS_EPOCH;

pub async fn configure_hasher() {
    let mut config = Config::new();

    let key = fs::read("secret.key")
        .expect("secret.key file does not exist")
        .into_boxed_slice();
    let key: &'static _ = Box::leak(key);

    config
        .set_secret_key(Some(key))
        .set_memory_cost(4096)
        .set_iterations(64);

    set_config(config).await;
}

pub fn get_system_rng() -> &'static SystemRandom {
    RNG.get_or_init(SystemRandom::new)
}

#[must_use]
pub fn get_epoch_time() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System clock is behind Unix Epoch")
        .as_millis()
        .saturating_sub(TOKEN_EPOCH)
}

// <id as string as b64>.<timestamp as string as b64>.<32 random bytes as b64>
#[must_use]
pub fn generate_token(user_id: u128) -> String {
    let mut token = encode_config(user_id.to_string().as_bytes(), URL_SAFE_NO_PAD);

    token.push('.');
    token.push_str(&encode_config(
        get_epoch_time().to_string().as_bytes(),
        URL_SAFE_NO_PAD,
    ));
    token.push('.');
    token.push_str(&{
        let dest = &mut [0_u8; 32];
        get_system_rng().fill(dest).expect("could not fill bytes");

        encode_config(dest, URL_SAFE_NO_PAD)
    });

    token
}

/// Represents authorization information from a request.
pub struct Auth(pub u128, pub String);

#[axum::async_trait]
impl FromRequest<Body> for Auth {
    type Rejection = HeaderAwareError;

    async fn from_request(req: &mut RequestParts<Body>) -> Result<Self, Self::Rejection> {
        let token = req
            .headers()
            .get("Authorization")
            .ok_or_else(|| {
                Response(
                    StatusCode::UNAUTHORIZED,
                    Error::<u128>::InvalidToken {
                        message: "Missing Authorization header, which should contain the token.",
                    },
                )
                .promote(req.headers())
            })?
            .to_str()
            .map_err(|_| {
                Response(
                    StatusCode::UNAUTHORIZED,
                    Error::<u128>::InvalidToken {
                        message: "Invalid Authorization header",
                    },
                )
                .promote(req.headers())
            })?;

        let id = crate::cache::resolve_token(req.headers(), token).await?;

        Ok(Self(id, token.to_string()))
    }
}
