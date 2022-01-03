use ferrischat_common::perms::ChannelPermissions;

use super::PermissionCalculatorTM;

impl PermissionCalculatorTM {
    pub fn to_channel(self) -> ChannelPermissions {
        let mut perms = ChannelPermissions::empty();

        let member = self.from_member;

        let mut roles = member.roles;

        roles.reverse();

        for role in roles {
            let perm = role.guild_permissions;

            perms.send_messages |= perm.send_messages;
            perms.delete_messages |= perm.delete_messages;
            perms.edit_channel |= perm.edit_channels;
        }

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
                    perms |= overwrite;
                }

                if roles.contains(&object) {
                    perms |= overwrite;
                }
            }
        }

        perms
    }
}
