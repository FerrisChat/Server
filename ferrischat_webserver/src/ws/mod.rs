mod info;

pub use info::ws_info;

use crate::WebServerError;
use ferrischat_common::types::Json;
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_redis::deadpool_redis::PoolError;
use ferrischat_redis::redis::{AsyncCommands, RedisError};

use axum::routing::get;
use axum::Router;

pub enum WsEventError {
    MissingRedis,
    RedisError(RedisError),
    JsonError(simd_json::Error),
    PoolError(PoolError),
}

pub async fn fire_event(channel: String, event: &WsOutboundEvent) -> Result<(), WebServerError> {
    let message = simd_json::to_vec(event).map_err(|e| WebServerError::Json(e))?;

    ferrischat_redis::REDIS_MANAGER
        .get()
        .ok_or(WebServerError::MissingRedis)?
        .get()
        .await
        .map_err(|e| WebServerError::RedisPool(e))?
        .publish::<_, _, Option<u32>>(channel, message)
        .await
        .map_err(|e| WebServerError::Redis(e))
        .map(|_| ())
}

pub fn generate_ws_route() -> axum::Router {
    Router::new()
        // GET    /ws/info
        .route(expand_version!("ws/info"), get(ws_info))
}
