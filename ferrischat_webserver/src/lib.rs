#![feature(once_cell)]
#![feature(type_alias_impl_trait)]
#![forbid(unsafe_code)]

#[cfg(not(target_os = "linux"))]
compile_error!("the server of FerrisChat is only supported on Linux systems");

#[macro_use]
extern crate ferrischat_macros;

mod auth;
mod channels;
mod entrypoint;
mod guilds;
mod members;
mod messages;
mod not_implemented;
mod users;
mod ws;

type GlobalHasher = std::lazy::SyncOnceCell<
    tokio::sync::mpsc::Sender<(
        String,
        tokio::sync::oneshot::Sender<Result<String, argonautica::Error>>,
    )>,
>;
type GlobalVerifier = std::lazy::SyncOnceCell<
    tokio::sync::mpsc::Sender<(
        (String, String),
        tokio::sync::oneshot::Sender<Result<bool, argonautica::Error>>,
    )>,
>;

pub const API_VERSION: u8 = 0;
pub static RNG_CORE: std::lazy::SyncOnceCell<ring::rand::SystemRandom> =
    std::lazy::SyncOnceCell::new();
pub static GLOBAL_HASHER: GlobalHasher = std::lazy::SyncOnceCell::new();
pub static GLOBAL_VERIFIER: GlobalVerifier = std::lazy::SyncOnceCell::new();

pub use auth::Authorization;
pub use entrypoint::*;
