mod bots;
mod create_user;
mod delete_user;
mod edit_user;
mod get_user;
mod verify_user;

pub use bots::*;
pub use create_user::*;
pub use delete_user::*;
pub use edit_user::*;
pub use get_user::*;
pub use verify_user::*;

use axum::routing::{delete, get, patch, post};
use axum::Router;

pub fn generate_users_route() -> axum::Router {
    Router::new()
        // POST   /users/
        .route(expand_version!("users"), post(create_user))
        // GET    /users/:user_id
        // PATCH  /users/:user_id
        // DELETE /users/:user_id
        .route(
            expand_version!("users/:user_id"),
            get(get_user).patch(edit_user).delete(delete_user),
        )
        // POST   /users/:user_id/bots
        // GET    /users/:user_id/bots
        .route(
            expand_version!("users/:user_id/bots"),
            post(create_bot).get(get_bots_by_user),
        )
        // PATCH  /users/:user_id/bots/:bot_id
        // DELETE /users/:user_id/bots/:bot_id
        .route(
            expand_version!("users/:user_id/bots/:bot_id"),
            patch(edit_bot).delete(delete_bot),
        )
}
