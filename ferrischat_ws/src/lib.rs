#![feature(once_cell)]
#![feature(async_closure)]

mod config;
mod events;
mod handle_connection;
mod init;
mod preload;
mod redis_handler;
mod rx_handler;
mod tx_handler;

use dashmap::DashMap;
use ferrischat_redis::redis::Msg;
pub use init::*;
use std::lazy::SyncOnceCell as OnceCell;
use uuid::Uuid;

#[macro_use]
extern crate ferrischat_macros;
#[macro_use]
extern crate tracing;

static USERID_CONNECTION_MAP: OnceCell<DashMap<Uuid, u128>> = OnceCell::new();

// ignore the name
static SUB_TO_ME: OnceCell<
    futures::channel::mpsc::Sender<(String, tokio::sync::mpsc::Sender<Option<Msg>>)>,
> = OnceCell::new();

pub enum TxRxComm {
    Text(String),
    Binary(Vec<u8>),
}
