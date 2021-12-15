use crate::WebServerError;
use ferrischat_common::types::{Channel, Guild, Invite, Member, Message, Role};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_redis::redis::AsyncCommands;

#[inline]
fn get_event_name(event: &WsOutboundEvent) -> String {
    match event {
        /*****************
         * Message Events *
         *****************/
        WsOutboundEvent::MessageDelete {
            message:
                Message {
                    channel: Channel { guild_id, .. },
                    ..
                },
        }
        | WsOutboundEvent::MessageUpdate {
            old:
                Message {
                    channel: Channel { guild_id, .. },
                    ..
                },
            ..
        }
        | WsOutboundEvent::MessageCreate {
            message:
                Message {
                    channel: Channel { guild_id, .. },
                    ..
                },
        } => format!("message_{}", *guild_id),

        /*****************
         * Channel Events *
         *****************/
        WsOutboundEvent::ChannelCreate {
            channel: Channel { guild_id, .. },
        }
        | WsOutboundEvent::ChannelUpdate {
            old: Channel { guild_id, .. },
            ..
        }
        | WsOutboundEvent::ChannelDelete {
            channel: Channel { guild_id, .. },
        } => format!("channel_{}", *guild_id),

        /***************
         * Guild Events *
         ***************/
        // GuildCreate is a special event
        // we are not subscribed to events for this guild because it was just created
        // so we need to dispatch events at the user level
        WsOutboundEvent::GuildCreate {
            guild: Guild { owner_id, .. },
        } => format!("gc_{}", owner_id),
        WsOutboundEvent::GuildUpdate {
            old: Guild { id, .. },
            ..
        }
        | WsOutboundEvent::GuildDelete {
            guild: Guild { id, .. },
        } => format!("guild_{}", id),

        /****************
         * Member Events *
         ****************/
        WsOutboundEvent::MemberCreate {
            member:
                Member {
                    guild_id: Some(guild_id),
                    ..
                }
                | Member {
                    guild: Some(Guild { id: guild_id, .. }),
                    ..
                },
        }
        | WsOutboundEvent::MemberUpdate {
            old:
                Member {
                    guild_id: Some(guild_id),
                    ..
                }
                | Member {
                    guild: Some(Guild { id: guild_id, .. }),
                    ..
                },
            ..
        }
        | WsOutboundEvent::MemberDelete {
            member:
                Member {
                    guild_id: Some(guild_id),
                    ..
                }
                | Member {
                    guild: Some(Guild { id: guild_id, .. }),
                    ..
                },
        } => format!("member_{}", guild_id),

        /****************
         * Invite Events *
         ****************/
        WsOutboundEvent::InviteCreate {
            invite: Invite { guild_id, .. },
        }
        | WsOutboundEvent::InviteDelete {
            invite: Invite { guild_id, .. },
        } => format!("invite_{}", guild_id),

        /**************
         * Role Events *
         **************/
        WsOutboundEvent::RoleCreate {
            role: Role { guild_id, .. },
        }
        | WsOutboundEvent::RoleUpdate {
            old: Role { guild_id, .. },
            ..
        }
        | WsOutboundEvent::RoleDelete {
            role: Role { guild_id, .. },
        } => format!("role_{}", guild_id),

        /****************
         * Typing Events *
         ****************/
        WsOutboundEvent::TypingStart {
            channel: Channel { guild_id, .. },
            ..
        }
        | WsOutboundEvent::TypingEnd {
            channel: Channel { guild_id, .. },
            ..
        } => format!("typing_{}", guild_id),

        /*********************
         * Member Role Events *
         *********************/
        WsOutboundEvent::MemberRoleAdd {
            role: Role { guild_id, .. },
            ..
        }
        | WsOutboundEvent::MemberRoleDelete {
            role: Role { guild_id, .. },
            ..
        } => format!("member_role_{}", guild_id),

        _ => panic!("called `fire_event` with an unsupported event type"),
    }
}

pub async fn fire_event(event: &WsOutboundEvent) -> Result<(), WebServerError> {
    let event_name = get_event_name(event);
    let message = simd_json::to_vec(event)?;

    ferrischat_redis::REDIS_MANAGER
        .get()
        .ok_or(WebServerError::MissingRedis)?
        .get()
        .await?
        .publish::<_, _, Option<u32>>(event_name, message)
        .await
        .map_err(WebServerError::from)
        .map(|_| ())
}
