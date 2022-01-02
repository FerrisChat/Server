#![feature(once_cell)]

use dashmap::DashMap;

pub mod calculate;
pub(crate) mod local_cache;
mod manage;

use ferrischat_common::perms::{ChannelPermissions, GuildPermissions};

pub enum Permissions {
    Guild(GuildPermissions),
    Channel(ChannelPermissions),
}

pub fn init_permissions() {
    crate::local_cache::LOCAL_PERMISSION_CACHE
        .set(DashMap::new())
        .unwrap_or_else(|_| panic!("don't call init_permissions() more than once!"));
}
