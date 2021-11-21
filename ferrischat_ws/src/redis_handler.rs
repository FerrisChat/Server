use ferrischat_redis::redis::{aio::PubSub, Msg};
use futures_util::StreamExt;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;
use tokio::time::Duration;

// this function is entirely self-contained: as long as it fires events properly it will work
pub async fn redis_event_handler(
    mut pubsub_conn: PubSub,
    mut rx: futures::channel::mpsc::Receiver<(String, Sender<Option<Msg>>)>,
) {
    let mut local_map: HashMap<_, Sender<Option<Msg>>> = HashMap::new();
    let mut to_unsub = Vec::new();
    loop {
        {
            debug!("polling for new messages");
            let mut s = pubsub_conn.on_message();
            for _ in 0..150 {
                const ONE_HUNDRED_MICROSECONDS: Duration = Duration::from_micros(100);
                match tokio::time::timeout(ONE_HUNDRED_MICROSECONDS, s.next()).await {
                    Ok(inner) => {
                        if let Some(x) = inner {
                            if let Ok(Some(pat)) = x.get_pattern::<Option<String>>() {
                                if let Some(sender) = local_map.get_mut(&pat) {
                                    if let Err(_) = sender.send(Some(x)).await {
                                        to_unsub.push(pat);
                                    };
                                }
                            }
                        } else {
                            break; // stream exhausted
                        }
                    }
                    Err(_) => {}
                }
            }
            // drop the stream, losing a &mut ref to it
        }
        // now poll up to 10x for more items in the new subscriptions category
        for _ in 0..10 {
            match rx.try_next() {
                Ok(Some((channel, output_queue))) => {
                    pubsub_conn.psubscribe(channel.clone()).await;
                    // FIXME: this breaks any existing connections subscribed to it
                    // ideas on how to fix:
                    // each UUID that wants to sub to an event is added to a map of events to UUIDs
                    // when a UUID gets an event get the sender from a map of UUIDs to senders and fire
                    // when a new event needs to be subscribed to, add it to a queue and have the Redis loop poll for updates from either Redis or the new sub queue
                    // if the new sub queue has a item, make sure Redis is flushed of items first, then break out and sub to that item
                    local_map.insert(channel, output_queue);
                }
                Ok(None) | Err(_) => break,
            }
        }
        // if there are any, remove nonexistent subscriptions
        for x in &to_unsub {
            pubsub_conn.punsubscribe(x).await;
        }
        // clear the vec
        to_unsub.clear();
    }
}
