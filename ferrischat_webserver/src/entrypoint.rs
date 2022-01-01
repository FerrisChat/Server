#![allow(clippy::wildcard_imports)]

use crate::auth::init_rng;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;

#[allow(clippy::expect_used)]
pub async fn entrypoint() {
    init_rng();

    let router = Router::new()
        // GET    /teapot
        .route(
            expand_version!("teapot"),
            get(async || (StatusCode::IM_A_TEAPOT, "")),
        )
        // GET    /ping
        .route(expand_version!("ping"), get(async || (StatusCode::OK, "pong")))
        .merge(crate::auth::generate_auth_routes())
        .merge(crate::channels::generate_channels_routes())
        .merge(crate::guilds::generate_guilds_routes())
        .merge(crate::invites::generate_invites_routes())
        .merge(crate::members::generate_members_routes())
        .merge(crate::messages::generate_messages_route())
        .merge(crate::users::generate_users_route())
        .merge(crate::ws::generate_ws_route());

    let listener = tokio::net::UnixListener::bind(format!(
        "{}/webserver.sock",
        std::env::var("FERRISCHAT_HOME").unwrap_or_else(|_| "/etc/ferrischat/".to_string())
    ))
    .expect("failed to bind to unix socket");
    let stream = tokio_stream::wrappers::UnixListenerStream::new(listener);
    let acceptor = hyper::server::accept::from_stream(stream);
    let server = axum::Server::builder(acceptor).serve(router.into_make_service()).with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.expect("failed to wait for ctrl+c: you will need to SIGTERM the server if you want it to shut down");
    });

    server.await.expect("failed to start HTTP server");
}
