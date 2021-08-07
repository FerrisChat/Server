#![feature(once_cell)]
#![feature(type_alias_impl_trait)]

mod auth;
mod channels;
mod entrypoint;
mod guilds;
mod members;
mod messages;
mod not_implemented;
mod users;

pub const API_VERSION: u8 = 0;
pub static RNG_CORE: OnceCell<ring::rand::SystemRandom> = OnceCell::new();

pub use entrypoint::*;
use std::lazy::SyncOnceCell as OnceCell;
