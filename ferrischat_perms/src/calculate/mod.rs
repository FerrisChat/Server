use ferrischat_common::types::{Channel, Guild, Member, Role};

pub mod channel;
pub mod guild;

pub struct PermissionCalculatorTM {
    pub from_member: &Member, // For Channel overwrites
    /// Guild channel
    pub to_channel: Option<&Channel>,
    /// Guild related
    pub to_guild: Option<&Guild>,
}

impl PermissionCalculatorTM {
    pub fn new(member: &Member) -> PermissionCalculatorTM {
        PermissionCalculatorTM {
            from_member: member,
            to_channel: None,
            to_guild: None,
        }
    }

    pub fn to_channel(self, channel: &Channel) -> PermissionCalculatorTM {
        PermissionCalculatorTM {
            to_channel: Some(channel),
            ..self
        }
    }

    pub fn to_guild(self, guild: &Guild) -> PermissionCalculatorTM {
        PermissionCalculatorTM {
            to_guild: Some(guild),
            ..self
        }
    }
}
