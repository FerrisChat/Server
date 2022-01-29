#![feature(once_cell)]
#![feature(async_closure)]
#![feature(box_syntax)]

use dashmap::DashMap;
pub use init::*;
use std::lazy::SyncOnceCell as OnceCell;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

mod config;
mod error_handling;
mod event;
mod events;
mod handle_connection;
mod init;
mod preload;
mod redis_handler;
mod rx_handler;
mod tx_handler;

#[macro_use]
extern crate ferrischat_macros;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate async_trait;

static USERID_CONNECTION_MAP: OnceCell<DashMap<Uuid, u128>> = OnceCell::new();

// ignore the name
static SUB_TO_ME: OnceCell<Sender<(String, Sender<Option<crate::event::RedisMessage>>)>> =
    OnceCell::new();
