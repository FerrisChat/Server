mod create_invite;
mod get_guild_invites;
mod get_invite;
mod use_invite;

pub use create_invite::*;
pub use get_guild_invites::*;
pub use get_invite::*;
pub use use_invite::*;

use axum::routing::{get, post};
use axum::Router;

pub fn generate_invites_routes() -> axum::Router {
    debug!("generating routes for invites");
    Router::new()
        // POST   /guilds/:guild_id/invites
        // GET    /guilds/:guild_id/invites
        .route(
            expand_version!("guilds/:guild_id/invites"),
            post(create_invite).get(get_guild_invites),
        )
        // GET    /invites/:code
        // POST   /invites/:code
        .route(
            expand_version!("invites/:code"),
            get(get_invite).post(use_invite),
        )
}
