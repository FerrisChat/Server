use clap::{app_from_crate, Arg};
use std::path::PathBuf;
use std::process::abort;
use tokio::runtime::{Builder, Runtime};
use tracing_subscriber::EnvFilter;

#[macro_use]
extern crate clap;

fn get_rt() -> Runtime {
    Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
}

fn load_config() {
    let app = app_from_crate!().arg(
        Arg::with_name("config")
            .validator(|val| {
                PathBuf::from(val).exists().then(|| ()).map_or(
                    Err("config file path provided is nonexistent".to_string()),
                    |_| Ok(()),
                )
            })
            .takes_value(true)
            .value_name("FILE")
            .help("Path to configuration file")
            .default_value("./config.toml"),
    );
    let matches = app.get_matches();
    let cfg_path = matches
        .value_of("config")
        .expect("unexpected missing config file path")
        .into();
    ferrischat_config::load_config(cfg_path);
}

fn check_cpu_features() {
    #[cfg(target_arch = "x86_64")]
    if !is_x86_feature_detected!("pclmulqdq") {
        eprintln!("Your CPU doesn't support `pclmulqdq`. Exiting.");
        abort()
    }
}

#[must_use]
pub fn init() -> Runtime {
    check_cpu_features();
    load_config();
    init_logging();
    get_rt()
}

pub async fn async_init() {
    ferrischat_auth::init_auth().await;
    ferrischat_redis::load_redis().await;
    ferrischat_db::load_db().await;
}

#[inline]
pub async fn start_http() {
    ferrischat_webserver::entrypoint().await;
}

#[inline]
pub async fn start_ws() {
    ferrischat_ws::init_ws_server().await;
}

pub async fn start_both() {
    let future1 = tokio::spawn(start_ws());
    let future2 = tokio::spawn(start_http());
    let (r1, r2) = futures::future::join(future1, future2).await;
    r1.expect("failed to spawn WebSocket server");
    r2.expect("failed to spawn HTTP server");
}
