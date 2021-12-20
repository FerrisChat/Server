mod fire_event;
mod info;

pub use fire_event::fire_event;
pub use info::ws_info;

use axum::routing::get;
use axum::Router;

pub fn generate_ws_route() -> axum::Router {
    debug!("generating routes for websockets");
    Router::new()
        // GET    /ws/info
        .route(expand_version!("ws/info"), get(ws_info))
}
