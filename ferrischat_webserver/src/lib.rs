#![feature(once_cell)]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]
#![deny(unsafe_code)]

#[cfg(not(any(target_os = "linux", target_os = "bsd")))]
compile_error!("the server of FerrisChat is only supported on Linux systems");

#[macro_use]
extern crate ferrischat_macros;

mod auth;
mod channels;
mod entrypoint;
mod guilds;
mod invites;
mod members;
mod messages;
mod not_implemented;
mod users;
mod ws;

pub const API_VERSION: u8 = 0;
pub static RNG_CORE: std::lazy::SyncOnceCell<ring::rand::SystemRandom> =
    std::lazy::SyncOnceCell::new();

pub use entrypoint::*;
pub use ferrischat_auth::Authorization;
