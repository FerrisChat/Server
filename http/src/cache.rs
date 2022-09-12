use axum::headers::HeaderMap;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::sync::OnceLock;

use crate::{get_pool, Error, HeaderAwareResult, PostgresU128, PromoteErr, Response, StatusCode};

static POOL: OnceLock<Pool> = OnceLock::new();

/// Creates a new Redis connection pool.
pub async fn setup() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_url("redis://localhost");
    let pool = cfg.create_pool(Some(Runtime::Tokio1))?;

    // Test connection
    redis::cmd("PING")
        .query_async::<_, ()>(&mut pool.get().await?)
        .await?;

    POOL.set(pool)
        .unwrap_or_else(|_| panic!("POOL.set called more than once")); // redis::aio::Connection doesn't implement Debug

    Ok(())
}

/// Resolves a token into a user ID.
pub async fn resolve_token(headers: &HeaderMap, token: &str) -> HeaderAwareResult<u128> {
    struct QueryResponse {
        user_id: PostgresU128,
    }

    if let Some(id) = POOL
        .get()
        .expect("Didn't call `cache::setup`")
        .get()
        .await
        .promote(headers)?
        .hget::<_, _, Option<u128>>("ferrischat_token_to_id", token)
        .await
        .promote(headers)?
    {
        return Ok(id);
    }

    let response: QueryResponse = sqlx::query_as!(
        QueryResponse,
        r#"SELECT user_id AS "user_id: PostgresU128" FROM tokens WHERE token = $1"#,
        token,
    )
    .fetch_optional(get_pool())
    .await
    .promote(headers)?
    .ok_or_else(|| {
        Response(
            StatusCode::UNAUTHORIZED,
            Error::InvalidToken {
                message: "Invalid authorization token",
            },
        )
        .promote(headers)
    })?;
    let id = response.user_id.to_u128();

    POOL.get()
        .expect("Didn't call `cache::setup`")
        .get()
        .await
        .promote(headers)?
        .hset::<_, _, _, ()>("ferrischat_token_to_id", token, id.to_string())
        .await
        .promote(headers)?;

    Ok(id)
}
