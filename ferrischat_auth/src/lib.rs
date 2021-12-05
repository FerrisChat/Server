#![feature(once_cell)]
#![feature(type_alias_impl_trait)]
#![allow(clippy::module_name_repetitions)]

mod init;
mod split_token;
mod verify_token;

pub use init::init_auth;
pub use split_token::*;
pub use verify_token::*;

#[macro_use]
extern crate ferrischat_macros;

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

pub static GLOBAL_HASHER: GlobalHasher = std::lazy::SyncOnceCell::new();
pub static GLOBAL_VERIFIER: GlobalVerifier = std::lazy::SyncOnceCell::new();
