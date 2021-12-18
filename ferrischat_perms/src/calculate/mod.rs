use ferrischat_common::perms::Permissions;

pub fn calculate_permissions(
    base: Permissions,
    roles: Vec<Permissions>,
    channel: Permissions,
) -> Permissions {
    // for each role calculate the permissions
}
