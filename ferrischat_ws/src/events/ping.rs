use crate::error_handling::WsEventHandlerError;
use crate::events::{RxEventData, RxHandlerData, WebSocketRxHandler};
use ferrischat_common::ws::WsOutboundEvent;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

pub struct PingEvent;

#[async_trait]
impl WebSocketRxHandler for PingEvent {
    async fn handle_event<'a, 'b>(
        _: &Pool<Postgres>,
        _: RxEventData,
        RxHandlerData { inter_tx, .. }: RxHandlerData<'a>,
        _: Uuid,
    ) -> Result<(), WsEventHandlerError<'b>> {
        inter_tx
            .send(WsOutboundEvent::Pong)
            .await
            .map_err(|_| WsEventHandlerError::Sender)
    }
}
