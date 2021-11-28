mod delete_message;
mod edit_message;
mod get_messages;
mod message_history;
mod send_message;

pub use delete_message::*;
pub use edit_message::*;
pub use get_messages::*;
pub use message_history::*;
pub use send_message::*;

use axum::routing::{get, post};
use axum::Router;

pub fn generate_messages_route() -> axum::Router {
    Router::new()
        // POST   /channels/:channel_id/messages
        // GET    /channels/:channel_id/messages
        .route(
            expand_version!("channels/:channel_id/messages"),
            post(create_message).get(get_message_history),
        )
        // GET    /channels/:channel_id/messages/:message_id
        // PATCH  /channels/:channel_id/messages/:message_id
        // DELETE /channels/:channel_id/messages/:message_id
        .route(
            expand_version!("channels/:channel_id/messages/:message_id"),
            get(get_message).patch(edit_message).delete(delete_message),
        )
}
