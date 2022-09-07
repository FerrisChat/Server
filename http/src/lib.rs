#![allow(clippy::module_name_repetitions)]
#![feature(try_blocks)]

pub mod response;
pub use response::{Error, HeaderAwareResponse, Response};

use axum::{http::StatusCode, routing::get, Router};
use dotenv::dotenv;
use std::net::SocketAddr;

use axum::http::HeaderMap;

/// Starts the HTTP webserver.
pub async fn start() {
    dotenv().ok();

    let router = Router::new()
        .route(
            "/",
            get(|| async { (StatusCode::OK, "Hello from FerrisChat") }),
        )
        .route("/teapot", get(|| async { StatusCode::IM_A_TEAPOT }))
        .route(
            "/test",
            get(|map: HeaderMap| async move {
                Response::ok(common::models::user::User {
                    id: 0,
                    username: "test".to_string(),
                    discriminator: 1234,
                    avatar: None,
                    banner: None,
                    flags: common::models::user::UserFlags::empty(),
                    bio: None,
                })
                .promote(&map)
            }),
        );

    let port = std::env::var("FERRISCHAT_WEBSERVER_PORT")
        .map(|port| port.parse::<u16>().expect("port should be a valid u16"))
        .unwrap_or(8080);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    axum::Server::bind(&addr)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("could not await Ctrl+C signal for graceful shutdown!");
        })
        .await
        .expect("could not start HTTP server");
}
