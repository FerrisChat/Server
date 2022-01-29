use crate::events::error::WebSocketHandlerError;
use dashmap::DashMap;
use ferrischat_common::ws::{Intents, WsOutboundEvent};
use sqlx::{Pool, Postgres};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub struct RxHandlerData {
    inter_tx: Sender<WsOutboundEvent>,
    uid_conn_map: Arc<DashMap<Uuid, u128>>,
    identify_received: Arc<AtomicBool>,
}

pub enum RxEventData {
    Identify { token: String, intents: Intents },
}

#[async_trait]
pub trait WebSocketRxHandler {
    /// Handle a event.
    ///
    /// Implementors can safely unwrap the event data they receive,
    /// as it is guaranteed to be of the correct type.
    async fn handle_event(
        db: &Pool<Postgres>,
        event_data: RxEventData,
        handler_data: RxHandlerData,
        conn_id: Uuid,
    ) -> Result<(), WebSocketHandlerError>;
}
