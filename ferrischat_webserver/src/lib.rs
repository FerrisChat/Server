#[macro_use]
extern crate rocket;

mod channels;
mod entrypoint;
mod guilds;
mod members;
mod messages;
mod not_implemented;
mod users;
mod version_expansion;

pub const API_VERSION: u8 = 0;

pub use entrypoint::*;
