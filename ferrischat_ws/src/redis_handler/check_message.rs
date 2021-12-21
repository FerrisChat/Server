use ferrischat_redis::redis_subscribe::Message;

#[allow(clippy::cognitive_complexity)]
pub(super) fn check_message(msg: &Message) -> bool {
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
