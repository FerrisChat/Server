mod auth_struct;
mod bot_get_token;
mod get_token;
mod init_rng;
mod token_gen;

pub use auth_struct::Authorization;
pub use bot_get_token::*;
pub use get_token::*;
pub use init_rng::*;
pub use token_gen::*;

use axum::routing::post;
use axum::Router;

pub fn generate_auth_routes() -> axum::Router {
    debug!("generating routes for auth");
    Router::new()
        // POST   /users/me/bots/:bot_id/auth
        .route(
            expand_version!("users/me/bots/:bot_id/auth"),
            post(get_bot_token),
        )
        // POST   /auth
        .route(expand_version!("auth"), post(get_token))
}
