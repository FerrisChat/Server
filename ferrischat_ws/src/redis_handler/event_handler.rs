use super::check_message::check_message;
use super::handle_message::handle_message;
use super::handle_subscribe::handle_subscribe;
use super::handle_unsubscribe::handle_unsubscribe;
use crate::event::RedisMessage;
use ahash::RandomState;
use dashmap::{DashMap, DashSet};
use ferrischat_redis::redis_subscribe::Message;
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};

// this function is entirely self-contained: as long as it fires events properly it will work
pub async fn event_handler(
    pubsub_conn: ferrischat_redis::redis_subscribe::RedisSub,
    mut rx: Receiver<(String, Sender<Option<RedisMessage>>)>,
) {
    let pubsub_conn = Arc::new(pubsub_conn);
    let to_unsub = Arc::new(DashSet::with_hasher(RandomState::new()));
    let event_channel_to_uuid_map = Arc::new(DashMap::with_hasher(RandomState::new()));
    let uuid_to_sender_map = Arc::new(DashMap::with_hasher(RandomState::new()));

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
            enum ResHandler {
                Redis(Option<Message>),
                NewSub(Option<(String, Sender<Option<RedisMessage>>)>),
            }

            let res: ResHandler = tokio::select! {
                item = s.next() => {
                    ResHandler::Redis(item)
                },
                item = rx.recv() => {
                    ResHandler::NewSub(item)
                }
            };

            match res {
                ResHandler::Redis(Some(redis_message)) => {
                    debug!("Redis returned an item: processing");

                    if check_message(&redis_message) {
                        continue;
                    };
                    if matches!(redis_message, Message::Disconnected(_)) {
                        break;
                    }

                    let p1 = Arc::clone(&event_channel_to_uuid_map);
                    let p2 = Arc::clone(&uuid_to_sender_map);
                    let p3 = Arc::clone(&to_unsub);
                    tokio::spawn(handle_message(p1, p2, p3, redis_message));
                }
                ResHandler::Redis(None) => warn!("Redis returned a empty message"),

                ResHandler::NewSub(Some(item)) => {
                    let p1 = Arc::clone(&pubsub_conn);
                    let p2 = Arc::clone(&event_channel_to_uuid_map);
                    let p3 = Arc::clone(&uuid_to_sender_map);
                    let (p4, p5) = item;

                    tokio::spawn(handle_subscribe(p1, p2, p3, p4, p5));
                }
                ResHandler::NewSub(None) => break 'outer,
            }

            if to_unsub.len() >= 5 {
                let p1 = Arc::clone(&pubsub_conn);
                let p2 = Arc::clone(&event_channel_to_uuid_map);
                let p3 = Arc::clone(&uuid_to_sender_map);
                let p4 = Arc::clone(&to_unsub);

                tokio::spawn(handle_unsubscribe(p1, p2, p3, p4));
            }
        }
    }
}
