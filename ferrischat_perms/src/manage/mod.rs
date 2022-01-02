use error::UpdatePermissionsError;
use crate::Permissions;
use ferrischat_common::types::Role;

mod error;

pub fn update_role_permissions(
    role_id: u128,
    new_permissions: Permissions,
) -> Result<(), UpdatePermissionsError> {
    Ok(())
}

pub fn update_user_roles(
    user_id: u128,
    role_list: Vec<Role>,
) -> Result<(), UpdatePermissionsError> {
    Ok(())
}
