#![feature(once_cell)]

use redis::aio::ConnectionManager;
use redis::Client;
use std::lazy::SyncOnceCell as OnceCell;

pub static REDIS_MANAGER: OnceCell<ConnectionManager> = OnceCell::new();

pub fn load_redis() -> ConnectionManager {
    let client = Client::open("redis://127.0.0.1:6379/").expect("initial redis connection failed");
    let manager = ConnectionManager::new(client)
        .await
        .expect("failed to open connection to Redis");
    REDIS_MANAGER.set(manager.clone()).unwrap_or_else(|_| {
        panic!("failed to set Redis global static: did you call load_redis() twice?")
    });
    manager
}
