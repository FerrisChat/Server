#![feature(once_cell)]

use serde::{Deserialize, Serialize};
use std::lazy::SyncOnceCell as OnceCell;

pub static GLOBAL_CONFIG: OnceCell<AppConfig> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    database: DatabaseConfig,
    redis: RedisConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RedisConfig {
    host: String,
    port: u16,
    user: Option<String>,
    password: Option<String>,
}

pub fn load_config(path: std::path::PathBuf) {
    let cfg_bytes =
        std::fs::read(path).expect("failed to load config: does it exist and is readable?");
    let cfg = toml::from_slice::<AppConfig>(&cfg_bytes[..])
        .expect("config deserialization failed: make sure there's no errors/missing fields");
    GLOBAL_CONFIG.set(cfg);
}
