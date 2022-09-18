#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::similar_names
)]
#![feature(is_some_with)]
#![feature(never_type)]
#![feature(once_cell)]
#![feature(try_blocks)]

pub mod auth;
pub mod cache;
pub mod checks;
pub mod database;
pub mod ratelimit;
pub mod response;
pub mod routes;

pub use auth::Auth;
pub use database::{get_pool, PostgresU128};
pub(crate) use ratelimit::ratelimit;
pub use response::{Error, HeaderAwareResponse, PromoteErr, Response};

pub type HeaderAwareError = HeaderAwareResponse<Error>;
pub type HeaderAwareResult<T> = Result<T, HeaderAwareError>;
pub type RouteResult<T> = HeaderAwareResult<HeaderAwareResponse<T>>;

use axum::{http::StatusCode, routing::get, Router};
use dotenv::dotenv;
use std::net::SocketAddr;

/// Starts the HTTP webserver.
pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    database::connect().await?;
    auth::configure_hasher().await;
    cache::setup().await?;

    let router = Router::new()
        .route(
            "/",
            get(|| async { (StatusCode::OK, "Hello from FerrisChat") }),
        )
        .route("/teapot", get(|| async { StatusCode::IM_A_TEAPOT }))
        .merge(routes::auth::router())
        .merge(routes::channel::router())
        .merge(routes::guild::router())
        .merge(routes::user::router());

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

    Ok(())
}
