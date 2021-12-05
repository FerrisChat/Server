#![feature(once_cell)]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]
#![deny(unsafe_code)]
#![allow(clippy::future_not_send)]
#![allow(clippy::module_name_repetitions)]

#[cfg(not(any(
    target_os = "linux",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
)))]
compile_error!(
    "the server of FerrisChat is only supported on Linux and BSD systems. \
    if your OS is supported but there's an issue, please email `os-support@ferris.chat`"
);

#[macro_use]
extern crate ferrischat_macros;

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
mod special_headers;
mod users;
mod ws;

pub const API_VERSION: u8 = 0;
pub static RNG_CORE: std::lazy::SyncOnceCell<ring::rand::SystemRandom> =
    std::lazy::SyncOnceCell::new();

pub(crate) use crate::auth::Authorization;
pub use entrypoint::*;
pub(crate) use errors::WebServerError;
pub(crate) use json_response::Json;
