mod add_member_role;
mod create_role;
mod delete_role;
mod edit_role;
mod get_role;
mod remove_member_role;

pub use add_member_role::*;
pub use create_role::*;
pub use delete_role::*;
pub use edit_role::*;
pub use get_role::*;
pub use remove_member_role::*;

use axum::routing::{get, post};
use axum::Router;

pub fn generate_roles_routes() -> axum::Router {
    Router::new()
        // POST   /guilds/:guild_id/roles
        .route(expand_version!("guilds/:guild_id/roles"), post(create_role))
        // GET    /guilds/:guild_id/roles/:role_id
        // DELETE /guilds/:guild_id/roles/:role_id
        // PATCH  /guilds/:guild_id/roles/:role_id
        .route(
            expand_version!("guilds/:guild_id/roles/:role_id"),
            get(get_role).delete(delete_role).patch(edit_role),
        )
        .route(
            expand_version!("guilds/:guild_id/members/:user_id/role/:role_id"),
            post(add_member_role).delete(remove_member_role),
        )
}
