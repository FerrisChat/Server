#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown
)]
#![feature(is_some_with)]
#![feature(once_cell)]
#![feature(try_blocks)]

pub mod database;
pub mod response;

pub use response::{Error, HeaderAwareResponse, Response};

use axum::{http::StatusCode, routing::get, Router};
use dotenv::dotenv;
use std::net::SocketAddr;

/// Starts the HTTP webserver.
pub async fn start() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    database::connect().await?;

    let router = Router::new()
        .route(
            "/",
            get(|| async { (StatusCode::OK, "Hello from FerrisChat") }),
        )
        .route("/teapot", get(|| async { StatusCode::IM_A_TEAPOT }));

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
