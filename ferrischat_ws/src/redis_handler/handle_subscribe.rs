use crate::event::RedisMessage;
use dashmap::{DashMap, DashSet};
use ferrischat_redis::redis_subscribe::RedisSub;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;

pub(super) async fn handle_subscribe(
    pubsub_conn: Arc<RedisSub>,
    uuid_to_sender_map: Arc<DashMap<Uuid, Sender<Option<RedisMessage>>>>,
    event_channel_to_uuid_map: Arc<DashMap<String, DashSet<Uuid>>>,
    channel: String,
    sender: Sender<Option<RedisMessage>>,
) {
    let channel_id = Uuid::new_v4();
    debug!(
        channel = %channel,
        uuid = %channel_id,
        "new subscriber detected"
    );

    if let Err(e) = pubsub_conn.psubscribe(channel.clone()).await {
        error!(
            channel = %channel,
            uuid = %channel_id,
            "failed to subscribe to Redis channel: {:?}",
            e
        );
        // drop the sender as a way of letting the other end know subscription failed
    } else {
        if let Some(x) = event_channel_to_uuid_map.get_mut(&channel) {
            debug!(
                channel = %channel,
                uuid = %channel_id,
                "new subscriber is being added to existing channel set"
            );
            x.insert(channel_id);
        } else {
            debug!(channel = %channel, uuid = %channel_id, "new subscriber is being added to new channel set");
            event_channel_to_uuid_map.insert(channel, {
                let s = DashSet::with_capacity(1);
                s.insert(channel_id);
                s
            });
        }

        assert!(uuid_to_sender_map.insert(channel_id, sender).is_none());
    }
}
