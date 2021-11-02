#![feature(once_cell)]

use ferrischat_config::GLOBAL_CONFIG;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Pool, Postgres};
use std::lazy::SyncOnceCell as OnceCell;
use std::time::Duration;

pub static DATABASE_POOL: OnceCell<Pool<Postgres>> = OnceCell::new();

pub async fn load_db() -> Pool<Postgres> {
    let cfg = GLOBAL_CONFIG
        .get()
        .expect("config not loaded: this is a bug");

    let db = PgPoolOptions::new()
        .max_connections(512)
        .min_connections(2)
        .max_lifetime(Some(Duration::from_secs(30 * 60)))
        .connect_with(
            PgConnectOptions::new()
                .database("ferris_chat")
                .username(&*cfg.database.user)
                .password(&*cfg.database.password)
                .host(&*cfg.database.host)
                .port(cfg.database.port)
                .statement_cache_capacity(1_048_576_usize),
        )
        .await
        // don't ask
        .unwrap_or_else(|_| panic!("failed to connect to DB"));

    DATABASE_POOL
        .set(db.clone())
        // also don't ask
        .unwrap_or_else(|_| panic!("failed to set the DB global: did you call load_db() twice?"));

    db
}
