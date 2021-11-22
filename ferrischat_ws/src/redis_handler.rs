use ferrischat_redis::redis::{aio::PubSub, Msg};
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

// this function is entirely self-contained: as long as it fires events properly it will work
pub async fn redis_event_handler(
    mut pubsub_conn: PubSub,
    mut rx: Receiver<(String, Sender<Option<Msg>>)>,
) {
    let mut to_unsub = Vec::new();

    let mut event_channel_to_uuid_map: HashMap<_, Vec<Uuid>> = HashMap::new();
    let mut uuid_to_sender_map = HashMap::new();

    loop {
        // process new Redis messages
        let new_sub: (String, Sender<Option<Msg>>) = {
            let mut s = pubsub_conn.on_message();
            loop {
                let redis_message: Msg = tokio::select! {
                    // we use biased here because Redis is the highest priority
                    biased;

                    item = s.next() => {
                        if let Some(item) = item {
                            debug!("Redis returned an item: processing")
                            item
                        } else {
                            continue
                        }
                    },
                    item = rx.recv() => {
                        if let Some(item) = item {
                            debug!("new subscriber detected: breaking out of redis poll loop");
                            break item
                        }
                        return
                    }
                };

                let event_channel = match redis_message.get_channel::<String>() {
                    Ok(c) => c,
                    Err(e) => {
                        error!(
                            "failed to parse Redis message channel name as String: {}",
                            e
                        );
                        continue;
                    }
                };
                debug!("redis new item has channel name {}", event_channel);

                if let Some(c) = event_channel_to_uuid_map.get(&event_channel) {
                    debug!(channel = event_channel, "channel name was found in the channel - uuid map");
                    for channel in c.iter() {
                        debug!(channel = event_channel, "uuid {} is subscribed", channel);
                        if let Some(c) = uuid_to_sender_map.get(channel) {
                            if Sender::send(c, Some(redis_message.clone())).await.is_err() {
                                debug!(channel = event_channel, uuid = channel, "failed to fire event, garbage collecting time");
                                to_unsub.push(*channel);
                            };
                        } else {
                            warn!(channel = event_channel, "uuid {} has no sender attached to it", channel);
                        }
                    }
                }
            }
        };

        // subscribe to new channels
        {
            let pubsub_conn = &mut pubsub_conn;
            let (channel, sender) = new_sub;
            let channel_id = Uuid::new_v4();
            debug!(channel = channel, uuid = uuid, "new subscriber detected");

            if let Err(e) = pubsub_conn.psubscribe(&channel).await {
                error!(channel = channel, uuid = uuid, "failed to subscribe to Redis channel: {}", e);
                // drop the sender as a way of letting the other end know subscription failed
            } else {
                if let Some(x) = event_channel_to_uuid_map.get_mut(&channel) {
                    debug!(
                        channel = channel,
                        uuid = uuid,
                        "new subscriber is being added to existing channel set"
                    );
                    x.push(channel_id);
                } else {
                    debug!(
                        channel = channel,
                        uuid = uuid,
                    )
                    event_channel_to_uuid_map.insert(channel, vec![channel_id]);
                }

                assert!(uuid_to_sender_map.insert(channel_id, sender).is_none());
            }
        }

        // if any, remove nonexistent subscriptions
        if !to_unsub.is_empty() {
            let mut positions = Vec::with_capacity(to_unsub.len());
            for (channel, map) in &mut event_channel_to_uuid_map {
                for x in &to_unsub {
                    if let Some(pos) = map.iter().position(|n| n == x) {
                        positions.push((channel.clone(), pos));
                        break;
                    }
                }
            }

            let mut unsub = Vec::with_capacity(positions.len());
            for (channel, idx) in positions {
                if let Some(x) = event_channel_to_uuid_map.get_mut(&channel) {
                    // we do not care about ordering, so we use this function which is O(1) not O(n)
                    x.swap_remove(idx);

                    if x.is_empty() {
                        unsub.push(channel.clone());
                    }
                }
            }
            for channel in unsub {
                event_channel_to_uuid_map.remove(&channel);
                if let Err(e) = pubsub_conn.punsubscribe(channel).await {
                    error!("failed to unsubscribe from Redis channel: {}", e);
                }
            }

            to_unsub.clear();
        }
    }
}
