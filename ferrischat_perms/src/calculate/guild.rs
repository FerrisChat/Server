use ferrischat_common::perms::GuildPermissions;

use super::PermissionCalculatorTM;

impl<'a> PermissionCalculatorTM<'a> {
    pub fn to_guild(self) -> GuildPermissions {
        let mut perms = GuildPermissions::empty();

        let mut roles = self.from_member.roles;

        roles.reverse();

        for role in roles {
            perms = perms | role.guild_permissions;
        }

        perms
    }
}
