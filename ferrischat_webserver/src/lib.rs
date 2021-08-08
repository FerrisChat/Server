#![feature(once_cell)]
#![feature(type_alias_impl_trait)]

#[cfg(not(target_os = "linux"))]
compile_error!("the server of FerrisChat is only supported on Linux systems");

mod auth;
mod channels;
mod entrypoint;
mod guilds;
mod members;
mod messages;
mod not_implemented;
mod users;

trait SizedErr: std::error::Error + std::marker::Sized {}

pub const API_VERSION: u8 = 0;
pub static RNG_CORE: std::lazy::SyncOnceCell<ring::rand::SystemRandom> =
    std::lazy::SyncOnceCell::new();
pub static GLOBAL_HASHER: std::lazy::SyncOnceCell<
    tokio::sync::mpsc::Sender<(
        String,
        tokio::sync::oneshot::Sender<Result<String, argonautica::Error>>,
    )>,
> = std::lazy::SyncOnceCell::new();
pub static GLOBAL_VERIFIER: std::lazy::SyncOnceCell<
    tokio::sync::mpsc::Sender<(
        (String, String),
        tokio::sync::oneshot::Sender<Result<bool, argonautica::Error>>,
    )>,
> = std::lazy::SyncOnceCell::new();

pub use auth::Authorization;
pub use entrypoint::*;
