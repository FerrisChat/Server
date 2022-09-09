use dotenv_codegen::dotenv;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::sync::OnceLock;

/// The global database pool.
pub static POOL: OnceLock<Pool<Postgres>> = OnceLock::new();

/// Connects to the database. This should only be called once.
pub async fn connect() -> Result<(), Box<dyn std::error::Error>> {
    let pool = PgPoolOptions::new()
        .connect(dotenv!(
            "DATABASE_URL",
            "DATABASE_URL environment variable not set"
        ))
        .await?;

    POOL.set(pool)
        .expect("cannot initialize database pool more than once");
    Ok(())
}

/// Retrieves the database pool.
#[must_use]
pub fn get_pool() -> &'static Pool<Postgres> {
    POOL.get().expect("database pool not initialized")
}

/// Migrates the database.
pub async fn migrate() {
    sqlx::migrate!("../migrations")
        .run(get_pool())
        .await
        .expect("could not run database migrations");
}

/// A workaround for PostgreSQL not support their own u128 type.
#[derive(sqlx::Type, Copy, Clone, Debug, PartialEq, Eq)]
#[sqlx(type_name = "u128")]
pub struct PostgresU128 {
    high: i64,
    low: i64,
}

impl PostgresU128 {
    /// Creates a new `PostgresU128` from a `u128`.
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub const fn new(value: u128) -> Self {
        Self {
            high: (value >> 64) as i64,
            low: value as i64,
        }
    }

    /// Converts the `PostgresU128` to a `u128`.
    #[allow(clippy::cast_sign_loss)]
    #[must_use]
    pub const fn to_u128(&self) -> u128 {
        ((self.high as u128) << 64) | (self.low as u128)
    }

    /// Maps the inner value to a new value.
    #[must_use]
    pub fn map(self, f: impl FnOnce(u128) -> u128) -> Self {
        Self::new(f(self.to_u128()))
    }
}

impl From<u128> for PostgresU128 {
    fn from(value: u128) -> Self {
        Self::new(value)
    }
}

impl From<PostgresU128> for u128 {
    fn from(value: PostgresU128) -> Self {
        value.to_u128()
    }
}
