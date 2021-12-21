use crate::event::RedisMessage;
use ferrischat_redis::redis_subscribe::{Message, RedisSub};
use futures_util::StreamExt;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::{Receiver, Sender};
use uuid::Uuid;

// this function is entirely self-contained: as long as it fires events properly it will work
pub async fn redis_event_handler(
    pubsub_conn: ferrischat_redis::redis_subscribe::RedisSub,
    mut rx: Receiver<(String, Sender<Option<RedisMessage>>)>,
) {
    let mut to_unsub = HashSet::new();
    let mut event_channel_to_uuid_map = HashMap::new();
    let mut uuid_to_sender_map = HashMap::new();

    'outer: loop {
        let mut s = pubsub_conn
            .listen()
            .await
            .expect("failed to open redis connection");
        let next: Message = s
            .next()
            .await
            .expect("just opened redis conn and already dead?");
        assert!(next.is_connected());
        loop {
            let redis_message: Message = tokio::select! {
                item = s.next() => {
                    if let Some(item) = item {
                        debug!("Redis returned an item: processing");
                        item
                    } else {
                        continue
                    }
                },
                item = rx.recv() => {
                    match item {
                        Some(item) => sub_to_new_channel(&pubsub_conn, item.0, item.1, &mut event_channel_to_uuid_map, &mut uuid_to_sender_map).await,
                        None => break 'outer,
                    }
                    continue
                }
            };

            if check_message(&redis_message) {
                continue;
            };

            let (pattern, channel, message) = match redis_message {
                Message::Message { channel, message } => {
                    warn!(%channel, "got normal message that should not be possible: {:?}", message);
                    continue;
                }
                Message::PatternMessage {
                    pattern,
                    channel,
                    message,
                } => (pattern, channel, message),
                Message::Disconnected(_) => break,
                _ => {
                    warn!("entered what should be unreachable code");
                    continue;
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
                    debug!(%pattern, %channel, "uuid {} is subscribed", uuid);
                    if let Some(c) = uuid_to_sender_map.get(uuid) {
                        if Sender::send(c, Some(msg.clone())).await.is_err() {
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

            if to_unsub.len() >= 5 {
                unsubscribe_from_channels(
                    &pubsub_conn,
                    &mut event_channel_to_uuid_map,
                    &mut uuid_to_sender_map,
                    &mut to_unsub,
                )
                .await;
            }
        }
    }
}

async fn unsubscribe_from_channels(
    pubsub_conn: &RedisSub,
    event_channel_to_uuid_map: &mut HashMap<String, HashSet<Uuid>>,
    uuid_to_sender_map: &mut HashMap<Uuid, Sender<Option<RedisMessage>>>,
    to_unsub: &mut HashSet<Uuid>,
) {
    // if any, remove nonexistent subscriptions

    let mut channels = Vec::with_capacity(to_unsub.len());
    'outer: for (channel, map) in &*event_channel_to_uuid_map {
        for x in &*to_unsub {
            if map.contains(x) {
                channels.push((channel.clone(), x));
                if to_unsub.len() == channels.len() {
                    break 'outer;
                }
            }
        }
    }

    let mut unsub = Vec::with_capacity(channels.len());
    for (channel, uuid) in channels {
        if let Some(x) = event_channel_to_uuid_map.get_mut(&channel) {
            x.remove(uuid);

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

    for uuid in &*to_unsub {
        if uuid_to_sender_map.remove(uuid).is_none() {
            warn!(channel = %uuid, "uuid not found in sender map");
        }
    }

    to_unsub.clear();
}

async fn sub_to_new_channel(
    pubsub_conn: &RedisSub,
    channel: String,
    sender: Sender<Option<RedisMessage>>,
    event_channel_to_uuid_map: &mut HashMap<String, HashSet<Uuid>>,
    uuid_to_sender_map: &mut HashMap<Uuid, Sender<Option<RedisMessage>>>,
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
                let mut s = HashSet::with_capacity(1);
                s.insert(channel_id);
                s
            });
        }

        assert!(uuid_to_sender_map.insert(channel_id, sender).is_none());
    }
}

#[allow(clippy::cognitive_complexity)]
fn check_message(msg: &Message) -> bool {
    match msg {
        Message::Subscription {
            channel,
            subscriptions,
        } => {
            debug!(%subscriptions, "subscribed to new channel {}", channel);
            true
        }
        Message::Unsubscription {
            channel,
            subscriptions,
        } => {
            debug!(%subscriptions, "unsubscribed from existing channel {}", channel);
            true
        }
        Message::Message { channel, .. } => {
            debug!(%channel, "got new message");
            false
        }

        Message::PatternSubscription {
            channel,
            subscriptions,
        } => {
            debug!(%subscriptions, "subscribed to new pattern {}", channel);
            true
        }
        Message::PatternUnsubscription {
            channel,
            subscriptions,
        } => {
            debug!(%subscriptions, "unsubscribed from existing pattern {}", channel);
            true
        }
        Message::PatternMessage {
            pattern, channel, ..
        } => {
            debug!(%channel, %pattern, "got new pattern message");
            false
        }

        Message::Connected => {
            info!("reconnected to Redis pubsub");
            true
        }
        Message::Disconnected(e) => {
            error!("disconnected from Redis pubsub: {:?}", e);
            false
        }
        Message::Error(e) => {
            error!("Redis pubsub error: {:?}", e);
            true
        }
    }
}
