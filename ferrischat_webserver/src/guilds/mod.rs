mod create_guild;
mod delete_guild;
mod edit_guild;
mod get_guild;
pub mod roles;

pub use create_guild::*;
pub use delete_guild::*;
pub use edit_guild::*;
pub use get_guild::*;

use axum::routing::{delete, get, patch, post};
use axum::Router;

pub fn generate_guilds_routes() -> axum::Router {
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
        // POST   /guilds/:guild_id/roles
        .route(
            expand_version!("guilds/:guild_id/roles"),
            post(roles::create_role),
        )
        // DELETE /guilds/:guild_id/roles/:role_id
        // PATCH  /guilds/:guild_id/roles/:role_id
        .route(
            expand_version!("guilds/:guild_id/roles/:role_id"),
            delete(roles::delete_role).patch(roles::edit_role),
        )
}
