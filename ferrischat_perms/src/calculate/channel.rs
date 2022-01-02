use ferrischat_common::perms::ChannelPermissions;

use super::PermissionCalculatorTM;

impl PermissionCalculatorTM {
    pub fn to_channel(self) -> ChannelPermissions {
        let mut perms = ChannelPermissions::empty();

        if let Some(mut roles) = self.roles {
            roles.reverse();

            for role in roles {
                perms = perms | role.guild_permissions;
            }
        }

        if let Some(member) = self.from_member {
            if let Some(channel) = self.to_channel {
                if let Some(overwrites) = channel.permission_overwrites {
                    let roles = member.roles.iter_mut().map(|x| x.id).collect::<Vec<u128>>();

                    for (object, overwrite) in overwrites {
                        if object == member.user_id {
                            perms = perms | overwrite;
                        }

                        if roles.contains(&object) {
                            perms = perms | overwrite;
                        }
                    }
                }
            }
        }

        perms
    }
}
