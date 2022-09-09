use argon2_async::{set_config, Config};
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
        .set_iterations(128);

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
