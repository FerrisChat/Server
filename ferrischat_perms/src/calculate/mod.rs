use ferrischat_common::types::{Channel, Guild, Member, Role};

pub mod channel;
pub mod guild;

pub struct PermissionCalculatorTM<'a> {
    pub from_member: &'a Member, // For Channel overwrites
    /// Guild channel
    pub to_channel: Option<&'a Channel>,
    /// Guild related
    pub to_guild: Option<&'a Guild>,
}

impl<'a> PermissionCalculatorTM<'a> {
    pub fn new(member: &'a Member) -> PermissionCalculatorTM {
        PermissionCalculatorTM {
            from_member: member,
            to_channel: None,
            to_guild: None,
        }
    }

    pub fn to_channel(self, channel: &'a Channel) -> PermissionCalculatorTM {
        PermissionCalculatorTM {
            to_channel: Some(channel),
            ..self
        }
    }

    pub fn to_guild(self, guild: &'a Guild) -> PermissionCalculatorTM {
        PermissionCalculatorTM {
            to_guild: Some(guild),
            ..self
        }
    }
}
