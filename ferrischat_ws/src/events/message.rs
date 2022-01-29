use super::tx::WebSocketTxHandler;
use crate::events::error::WebSocketHandlerError;
use ferrischat_common::ws::WsOutboundEvent;
use sqlx::{Pool, Postgres};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub struct MessageEvent;

#[async_trait]
impl WebSocketTxHandler for MessageEvent {
    async fn handle_event(
        db: &Pool<Postgres>,
        msg: &WsOutboundEvent,
        user_id: u128,
        object_id: u128,
    ) -> Result<bool, WebSocketHandlerError> {
        // FIXME: once implemented, do a query to check the user has permissions to read messages in here
        let bigint_user_id = u128_to_bigdecimal!(user_id);
        let bigint_guild_id = u128_to_bigdecimal!(guild_id);

        sqlx::query!(
            "SELECT guild_id FROM members WHERE user_id = $1 AND guild_id = $2",
            bigint_user_id,
            bigint_guild_id
        )
        .fetch_optional(db)
        .await?
        .map(|_| true)
        .unwrap_or(false)
    }
}
