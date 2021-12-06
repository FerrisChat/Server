#![feature(once_cell)]
#![feature(type_alias_impl_trait)]
#![allow(clippy::module_name_repetitions)]

mod init;
mod split_token;
mod verify_token;

pub use argon2_async::{hash, verify, Error as Argon2Error};
pub use init::init_auth;
pub use split_token::*;
pub use verify_token::*;

#[macro_use]
extern crate ferrischat_macros;
