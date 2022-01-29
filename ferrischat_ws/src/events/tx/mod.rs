use crate::events::error::WebSocketHandlerError;
use ferrischat_common::ws::WsOutboundEvent;
use sqlx::{Pool, Postgres};

#[async_trait]
pub trait WebSocketTxHandler {
    async fn handle_event(
        db: &Pool<Postgres>,
        msg: &WsOutboundEvent,
        user_id: u128,
        object_id: u128,
    ) -> Result<bool, WebSocketHandlerError>;
}
