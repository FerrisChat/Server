mod bots;
mod create_user;
mod delete_user;
mod edit_user;
mod get_user;
mod verify_user;
mod get_me;

pub use bots::*;
pub use create_user::*;
pub use delete_user::*;
pub use edit_user::*;
pub use get_user::*;
pub use verify_user::*;
pub use get_me::*;

use axum::routing::{get, patch, post};
use axum::Router;

pub fn generate_users_route() -> axum::Router {
    debug!("generating routes for users");
    Router::new()
        //GET /users/me
        .route(expand_version!("users/me"), get(get_me))
        // POST   /users/
        .route(expand_version!("users"), post(create_user))
        // GET    /users/:user_id
        .route(expand_version!("users/:user_id"), get(get_user))
        // PATCH  /users/me
        // DELETE /users/me
        .route(
            expand_version!("users/me"),
            patch(edit_user).delete(delete_user),
        )
        // POST   /verify
        .route(expand_version!("verify"), post(send_verification_email))
        // GET    /verify/:token
        .route(expand_version!("verify/:token"), get(verify_email))
        // POST   /users/:user_id/bots
        // GET    /users/:user_id/bots
        .route(
            expand_version!("users/me/bots"),
            post(create_bot).get(get_bots_by_user),
        )
        // PATCH  /users/:user_id/bots/:bot_id
        // DELETE /users/:user_id/bots/:bot_id
        .route(
            expand_version!("users/me/bots/:bot_id"),
            patch(edit_bot).delete(delete_bot),
        )
        // POST /bots/:bot_id/add
        .route(expand_version!("bots/:bot_id/add"), post(invite_bot))
}
