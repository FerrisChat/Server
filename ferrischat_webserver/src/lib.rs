#![feature(once_cell)]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]
#![deny(unsafe_code)]
#![allow(clippy::future_not_send)]
#![allow(clippy::module_name_repetitions)]

#[cfg(target_os = "windows")]
compile_error!(
    "The server of FerrisChat is not supported on Windows/NT based systems.\
    If your OS is supported, but you are seeing this, please either email `os-support@ferris.chat`\
    or open an issue on our GitHub (https://github.com/FerrisChat/Server)"
);

#[macro_use]
extern crate ferrischat_macros;

#[macro_use]
extern crate tracing;

mod auth;
mod channels;
mod entrypoint;
mod errors;
mod guilds;
mod invites;
mod json_response;
mod members;
mod messages;
mod not_implemented;
mod users;
mod ws;

pub const API_VERSION: u8 = 0;
pub static RNG_CORE: std::lazy::SyncOnceCell<ring::rand::SystemRandom> =
    std::lazy::SyncOnceCell::new();

pub(crate) use crate::auth::Authorization;
pub use entrypoint::*;
pub(crate) use errors::WebServerError;
pub(crate) use json_response::Json;
