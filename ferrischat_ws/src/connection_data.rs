use std::sync::atomic::AtomicBool;
use uuid::Uuid;

pub struct WebSocketConnectionData {
    pub identify_recieved: AtomicBool,
    pub connection_id: Uuid,
}
