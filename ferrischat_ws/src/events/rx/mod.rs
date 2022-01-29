use crate::error_handling::WsEventHandlerError;
use dashmap::DashMap;
use ferrischat_common::ws::{Intents, WsOutboundEvent};
use sqlx::{Pool, Postgres};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub struct RxHandlerData<'a> {
    pub inter_tx: Sender<WsOutboundEvent>,
    pub uid_conn_map: &'a DashMap<Uuid, u128>,
    pub identify_received: Arc<AtomicBool>,
}

pub enum RxEventData {
    Identify { token: String, intents: Intents },
    Ping,
    Pong,
}

#[async_trait]
pub trait WebSocketRxHandler {
    /// Handle a event.
    ///
    /// Implementors can safely unwrap the event data they receive,
    /// as it is guaranteed to be of the correct type.
    async fn handle_event<'a, 'b>(
        db: &Pool<Postgres>,
        event_data: RxEventData,
        handler_data: RxHandlerData<'a>,
        conn_id: Uuid,
    ) -> Result<(), WsEventHandlerError<'b>>;
}
