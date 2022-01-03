use dashmap::DashMap;
use std::lazy::SyncOnceCell as OnceCell;

use crate::Permissions;

pub enum LocalCacheError {
    CacheMissing,
    ItemMissing,
}

pub(super) static LOCAL_PERMISSION_CACHE: OnceCell<DashMap<u128, Permissions>> = OnceCell::new();

/// Get an item from the local permissions cache.
///
/// Return the item or an error if not found.
pub fn get(k: &u128) -> Result<&Permissions, LocalCacheError> {
    Ok(LOCAL_PERMISSION_CACHE
        .get()
        .ok_or(LocalCacheError::CacheMissing)?
        .get(k)
        .ok_or(LocalCacheError::ItemMissing)?
        .value())
}

/// Get a mutable reference to an entry in the map
pub fn get_mut(k: &u128) -> Result<&mut Permissions, LocalCacheError> {
    Ok(LOCAL_PERMISSION_CACHE
        .get()
        .ok_or(LocalCacheError::CacheMissing)?
        .get_mut(k)
        .ok_or(LocalCacheError::ItemMissing)?
        .value_mut())
}

/// Get an owned object from an entry in the map.
///
/// Updating the owned object will not update the item in the map.
pub fn get_owned(k: &u128) -> Result<Permissions, LocalCacheError> {
    Ok(*LOCAL_PERMISSION_CACHE
        .get()
        .ok_or(LocalCacheError::CacheMissing)?
        .get(k)
        .ok_or(LocalCacheError::ItemMissing)?
        .value())
}

/// Inserts a key and a value into the map. Returns the old value associated with the key if there was one.
pub fn set(k: u128, v: Permissions) -> Result<Option<Permissions>, LocalCacheError> {
    Ok(LOCAL_PERMISSION_CACHE
        .get()
        .ok_or(LocalCacheError::CacheMissing)?
        .insert(k, v))
}

/// Removes an entry from the map, returning the key and value if the entry existed and the
/// provided conditional function returned true.
pub fn remove_if(
    k: &u128,
    f: impl FnOnce(&u128, &Permissions) -> bool,
) -> Result<Option<(u128, Permissions)>, LocalCacheError> {
    Ok(LOCAL_PERMISSION_CACHE
        .get()
        .ok_or(LocalCacheError::CacheMissing)?
        .remove_if(k, f))
}

/// Modify a specific value according to a function.
///
/// **DANGER**: DO NOT panic, or even allow possibility for a panic, inside the function.
/// If the function panics, the entire process will abort which is not good.
pub fn alter(
    k: &u128,
    f: impl FnOnce(&u128, Permissions) -> Permissions,
) -> Result<(), LocalCacheError> {
    Ok(LOCAL_PERMISSION_CACHE
        .get()
        .ok_or(LocalCacheError::CacheMissing)?
        .alter(k, f))
}
