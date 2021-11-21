#![feature(once_cell)]
#![feature(async_closure)]

use std::lazy::SyncOnceCell as OnceCell;

use dashmap::DashMap;
use uuid::Uuid;

use ferrischat_redis::redis::Msg;
pub use init::*;

mod config;
mod error_handling;
mod events;
mod handle_connection;
mod init;
mod inter_communication;
mod preload;
mod redis_handler;
mod rx_handler;
mod tx_handler;
mod types;

#[macro_use]
extern crate ferrischat_macros;
#[macro_use]
extern crate tracing;

static USERID_CONNECTION_MAP: OnceCell<DashMap<Uuid, u128>> = OnceCell::new();

// ignore the name
static SUB_TO_ME: OnceCell<
    tokio::sync::mpsc::Sender<(String, tokio::sync::mpsc::Sender<Option<Msg>>)>,
> = OnceCell::new();
