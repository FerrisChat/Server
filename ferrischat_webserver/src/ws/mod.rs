mod info;

pub use info::ws_info;

use crate::WebServerError;
use axum::routing::get;
use axum::Router;
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_redis::redis::AsyncCommands;

pub async fn fire_event(channel: String, event: &WsOutboundEvent) -> Result<(), WebServerError> {
    let message = simd_json::to_vec(event)?;

    ferrischat_redis::REDIS_MANAGER
        .get()
        .ok_or(WebServerError::MissingRedis)?
        .get()
        .await?
        .publish::<_, _, Option<u32>>(channel, message)
        .await
        .map_err(WebServerError::from)
        .map(|_| ())
}

pub fn generate_ws_route() -> axum::Router {
    debug!("generating routes for websockets");
    Router::new()
        // GET    /ws/info
        .route(expand_version!("ws/info"), get(ws_info))
}
