#![feature(once_cell)]

mod channels;
mod entrypoint;
mod get_db_or_fail;
mod guilds;
mod members;
mod messages;
mod not_implemented;
mod users;
mod version_expansion;

pub const API_VERSION: u8 = 0;

pub use entrypoint::*;
