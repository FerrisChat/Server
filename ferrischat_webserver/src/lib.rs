#![feature(once_cell)]

mod channels;
mod entrypoint;
mod guilds;
mod members;
mod messages;
mod not_implemented;
mod users;

pub const API_VERSION: u8 = 0;

pub use entrypoint::*;
