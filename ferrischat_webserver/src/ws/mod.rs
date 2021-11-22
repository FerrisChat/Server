mod info;

use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_redis::redis::{AsyncCommands, RedisError};
pub use info::ws_info;

pub enum WsEventError {
    MissingRedis,
    RedisError(RedisError),
    JsonError(simd_json::Error),
}

pub async fn fire_event(channel: String, event: &WsOutboundEvent) -> Result<(), WsEventError> {
    match ferrischat_redis::REDIS_MANAGER.get() {
        Some(mgr) => {
            match mgr
                .clone()
                .publish::<_, _, Option<u32>>(
                    channel,
                    match simd_json::to_vec(event) {
                        Ok(msg) => msg,
                        Err(e) => return Err(WsEventError::JsonError(e)),
                    },
                )
                .await
            {
                Ok(_) => Ok(()),
                Err(e) => Err(WsEventError::RedisError(e)),
            }
        }
        None => Err(WsEventError::MissingRedis),
    }
}
