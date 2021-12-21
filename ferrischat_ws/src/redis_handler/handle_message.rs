use crate::event::RedisMessage;
use ahash::RandomState;
use dashmap::{DashMap, DashSet};
use ferrischat_redis::redis_subscribe::Message;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub(super) async fn handle_message(
    event_channel_to_uuid_map: Arc<DashMap<String, DashSet<Uuid>, RandomState>>,
    uuid_to_sender_map: Arc<DashMap<Uuid, Sender<Option<RedisMessage>>, RandomState>>,
    to_unsub: Arc<DashSet<Uuid, RandomState>>,
    redis_message: Message,
) {
    let (pattern, channel, message) = match redis_message {
        Message::Message { channel, message } => {
            warn!(%channel, "got normal message that should not be possible: {:?}", message);
            return;
        }
        Message::PatternMessage {
            pattern,
            channel,
            message,
        } => (pattern, channel, message),
        Message::Disconnected(_) => return,
        _ => {
            warn!("entered what should be unreachable code");
            return;
        }
    };
    debug!(%pattern, %channel, "got new item data");
    let msg = RedisMessage {
        channel: channel.clone(),
        message,
    };

    if let Some(c) = event_channel_to_uuid_map.get(&pattern) {
        debug!(
            %pattern, %channel,
            "channel name was found in the channel - uuid map"
        );
        for uuid in c.iter() {
            let uuid = &*uuid;
            debug!(%pattern, %channel, "uuid {} is subscribed", uuid);
            if let Some(c) = uuid_to_sender_map.get(uuid) {
                if Sender::send(c.value(), Some(msg.clone())).await.is_err() {
                    debug!(
                        %pattern, %channel, %uuid,
                        "failed to fire event, garbage collecting time"
                    );
                    to_unsub.insert(*uuid);
                };
            } else {
                warn!(
                    %pattern, %channel, %uuid,
                    "uuid has no sender attached to it"
                );
            }
        }
    }
}
