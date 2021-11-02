#![feature(once_cell)]

use serde::{Deserialize, Serialize};
use std::lazy::SyncOnceCell as OnceCell;

pub static GLOBAL_CONFIG: OnceCell<AppConfig> = OnceCell::new();

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
}

pub fn load_config(path: std::path::PathBuf) {
    let cfg_bytes =
        std::fs::read(path).expect("failed to load config: does it exist and is readable?");
    let cfg = toml::from_slice::<AppConfig>(&cfg_bytes[..])
        .expect("config deserialization failed: make sure there's no errors/missing fields");
    GLOBAL_CONFIG.set(cfg);
}
