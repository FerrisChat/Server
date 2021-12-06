mod create_guild;
mod delete_guild;
mod edit_guild;
mod get_guild;
pub mod roles;

pub use create_guild::*;
pub use delete_guild::*;
pub use edit_guild::*;
pub use get_guild::*;

use axum::routing::{get, post};
use axum::Router;

pub fn generate_guilds_routes() -> axum::Router {
    debug!("generating routes for guilds");
    Router::new()
        // POST   /guilds
        .route(expand_version!("guilds"), post(create_guild))
        // GET    /guilds/:guild_id
        // PATCH  /guilds/:guild_id
        // DELETE /guilds/:guild_id
        .route(
            expand_version!("guilds/:guild_id"),
            get(get_guild).patch(edit_guild).delete(delete_guild),
        )
        // roles routes
        .merge(roles::generate_roles_routes())
}
