use crate::event::RedisMessage;
use dashmap::{DashMap, DashSet};
use ferrischat_redis::redis_subscribe::RedisSub;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub(super) async fn handle_unsubscribe(
    pubsub_conn: Arc<RedisSub>,
    event_channel_to_uuid_map: Arc<DashMap<String, DashSet<Uuid>>>,
    uuid_to_sender_map: Arc<DashMap<Uuid, Sender<Option<RedisMessage>>>>,
    to_unsub: Arc<DashSet<Uuid>>,
) {
    let mut channels = Vec::with_capacity(to_unsub.len());
    'outer: for i in event_channel_to_uuid_map.iter() {
        let (channel, map) = i.pair();
        for x in to_unsub.iter() {
            if map.contains(&*x) {
                channels.push((channel.clone(), *x));
                if to_unsub.len() == channels.len() {
                    break 'outer;
                }
            }
        }
    }

    let mut unsub = Vec::with_capacity(channels.len());
    for (channel, uuid) in channels {
        if let Some(x) = event_channel_to_uuid_map.get_mut(&channel) {
            x.remove(&uuid);

            if x.is_empty() {
                // we're using a mutable ref in the loop so we can't just remove it here
                unsub.push(channel.clone());
            }
        }
    }
    for channel in unsub {
        event_channel_to_uuid_map.remove(&channel);
        if let Err(e) = pubsub_conn.punsubscribe(channel).await {
            error!("failed to unsubscribe from Redis channel: {:?}", e);
        }
    }

    for uuid in to_unsub.iter() {
        let uuid = &*uuid;
        if uuid_to_sender_map.remove(uuid).is_none() {
            warn!(channel = %uuid, "uuid not found in sender map");
        }
    }

    to_unsub.clear();
}
