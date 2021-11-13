#![feature(once_cell)]

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
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

impl Display for RedisConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("redis://")?;
        if let Some(ref user) = self.user {
            f.write_str(user)?;
        }
        if let Some(ref password) = self.password {
            f.write_str(":")?;
            f.write_str(password)?;
            f.write_str("@")?;
        }
        f.write_str(&*self.host)?;
        f.write_str(":")?;
        f.write_str(&*self.port.to_string())?;
        Ok(())
    }
}

pub fn load_config(path: std::path::PathBuf) {
    let cfg_bytes =
        std::fs::read(path).expect("failed to load config: does it exist and is readable?");
    let cfg = toml::from_slice::<AppConfig>(&cfg_bytes[..])
        .expect("config deserialization failed: make sure there's no errors/missing fields");
    GLOBAL_CONFIG
        .set(cfg)
        .unwrap_or_else(|_| panic!("config was already loaded: did you call load_config() twice?"));
}
