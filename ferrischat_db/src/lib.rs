#![feature(once_cell)]

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Pool, Postgres};
use std::lazy::SyncOnceCell as OnceCell;
use std::path::PathBuf;
use std::time::Duration;

pub static DATABASE_POOL: OnceCell<Pool<Postgres>> = OnceCell::new();

pub async fn load_db() -> Pool<Postgres> {
    let db = PgPoolOptions::new()
        .max_connections(8_192)
        .min_connections(32)
        .max_lifetime(Some(Duration::from_secs(3600)))
        .connect_with(
            PgConnectOptions::new()
                .database("ferris_chat")
                .username("ferris_chat")
                .password("ferris_chat")
                .host("localhost")
                .statement_cache_capacity(1_048_576_usize),
        )
        .await
        .expect("failed to connect to the database");

    DATABASE_POOL
        .set(db.clone())
        .expect("failed to set the DB global: did you call load_db() twice?");

    db
}
