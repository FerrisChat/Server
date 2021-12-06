// TODO: add arguments for guild to each function here

mod delete_member;
mod get_member;

pub use delete_member::*;
pub use get_member::*;

use axum::routing::get;
use axum::Router;

pub fn generate_members_routes() -> axum::Router {
    debug!("generating routes for members");
    Router::new()
        // GET    /guilds/:guild_id/members/:member_id
        // PATCH  /guilds/:guild_id/members/:member_id
        // DELETE /guilds/:guild_id/members/:member_id
        .route(
            expand_version!("guilds/:guild_id/members/:member_id"),
            get(get_member)
                .patch(crate::not_implemented::not_implemented)
                .delete(delete_member),
        )
}
