use ferrischat_common::perms::ChannelPermissions;

use super::PermissionCalculatorTM;

impl<'a> PermissionCalculatorTM<'a> {
    pub fn to_channel(self) -> ChannelPermissions {
        let mut perms = ChannelPermissions::empty();

        let mut roles = self.from_member.roles;

        roles.reverse();

        for role in roles {
            perms = perms | role.guild_permissions;
        }

        let member = self.from_member;

        if let Some(channel) = self.to_channel {
            let mut overwrites = channel.permission_overwrites;
            let roles = roles.iter_mut().map(|x| x.id).collect::<Vec<u128>>();

            overwrites.reverse();

            let user_id = match member.user_id {
                Some(id) => id,
                None => {
                    member
                        .user
                        .unwrap_or_else(|| unreachable!("No user_id and no user"))
                        .id
                }
            };

            for (object, overwrite) in overwrites {
                if object == user_id {
                    perms = perms | overwrite;
                }

                if roles.contains(&object) {
                    perms = perms | overwrite;
                }
            }
        }

        perms
    }
}
