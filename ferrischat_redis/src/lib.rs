#![feature(once_cell)]

pub use redis;
use redis::aio::{ConnectionManager, PubSub};
use redis::{Client, RedisResult};
use std::lazy::SyncOnceCell as OnceCell;

pub static REDIS_MANAGER: OnceCell<ConnectionManager> = OnceCell::new();
pub const REDIS_LOCATION: &'static str = "redis://127.0.0.1:6379/";

pub async fn load_redis() -> ConnectionManager {
    let client = Client::open(REDIS_LOCATION).expect("initial redis connection failed");
    let manager = ConnectionManager::new(client)
        .await
        .expect("failed to open connection to Redis");
    REDIS_MANAGER.set(manager.clone()).unwrap_or_else(|_| {
        panic!("failed to set Redis global static: did you call load_redis() twice?")
    });
    manager
}

pub async fn get_pubsub() -> RedisResult<PubSub> {
    Ok(Client::open(REDIS_LOCATION)?
        .get_tokio_connection()
        .await?
        .into_pubsub())
}
