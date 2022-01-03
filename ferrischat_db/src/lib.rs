#![feature(once_cell)]

pub use sqlx;

use ferrischat_config::GLOBAL_CONFIG;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Pool, Postgres};
use std::lazy::SyncOnceCell as OnceCell;
use std::time::Duration;

pub static DATABASE_POOL: OnceCell<Pool<Postgres>> = OnceCell::new();

/// Load the Postgres pool, set it into the global database pool, and return it.
///
/// # Panics
/// If the global pool was already set.
/// This will only happen if this function is called more than once.
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

/// Run all pending migrations against the database.
///
/// # Panics
/// If loading the database pool or running the actual migrations failed.
pub async fn run_migrations() {
    let db = load_db().await;
    let migrator: sqlx::migrate::Migrator = sqlx::migrate!("./../migrations");
    migrator.run(&db).await.expect("failed to run migrations");
}
