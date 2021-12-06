use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

/// Maximum number of messages to buffer in the `WebSocket` send queue.
const MAX_SEND_QUEUE: usize = 32_768;
/// Maximum size of a `WebSocket` message.
const MAX_MESSAGE_SIZE: usize = 67_108_864; // 64 MiB
/// Maximum size of a single `WebSocket` frame.
const MAX_FRAME_SIZE: usize = 16_777_216; // 16 MiB

pub const WEBSOCKET_CONFIG: WebSocketConfig = WebSocketConfig {
    max_send_queue: Some(MAX_SEND_QUEUE),
    max_message_size: Some(MAX_MESSAGE_SIZE),
    max_frame_size: Some(MAX_FRAME_SIZE),
    accept_unmasked_frames: false,
};
