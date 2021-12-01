mod create_channel;
mod delete_channel;
mod edit_channel;
mod get_channel;
mod typing_end;
mod typing_start;

pub use create_channel::*;
pub use delete_channel::*;
pub use edit_channel::*;
pub use get_channel::*;
pub use typing_end::*;
pub use typing_start::*;

use axum::routing::{get, post};
use axum::Router;

pub fn generate_channels_routes() -> axum::Router {
    Router::new()
        // POST   /guilds/:guild_id/channels
        .route(
            expand_version!("guilds/:guild_id/channels"),
            post(create_channel),
        )
        // GET    /channels/:channel_id
        // PATCH  /channels/:channel_id
        // DELETE /channels/:channel_id
        .route(
            expand_version!("channels/:channel_id"),
            get(get_channel).patch(edit_channel).delete(delete_channel),
        )
        // POST   /channels/:channel_id/typing
        // DELETE /channels/:channel_id/typing
        .route(
            expand_version!("channels/:channel_id/typing"),
            post(typing_start).delete(typing_end),
        )
}
