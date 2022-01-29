use crate::error_handling::WsEventHandlerError;
use crate::events::error::WebSocketHandlerError;
use crate::events::rx::{RxEventData, RxHandlerData, WebSocketRxHandler};
use crate::events::utils::bigdecimal_to_u128;
use dashmap::DashMap;
use ferrischat_auth::{split_token, verify_token};
use ferrischat_common::types::UserFlags;
use ferrischat_common::ws::{Intents, WsOutboundEvent};
use num_traits::ToPrimitive;
use sqlx::{Pool, Postgres};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use uuid::Uuid;

struct IdentifyEvent;

impl WebSocketRxHandler for IdentifyEvent {
    async fn handle_event(
        db: &Pool<Postgres>,
        event_data: RxEventData,
        RxHandlerData {
            inter_tx,
            uid_conn_map,
            identify_received,
        }: RxHandlerData,
        conn_id: Uuid,
    ) -> Result<(), WebSocketHandlerError> {
        #[allow(unreachable_patterns)]
        let (token, intents) = match event_data {
            RxEventData::Identify { token, intents } => (token, intents),
            _ => unreachable!("got wrong event type in Identify"),
        };

        if identify_received.swap(true, Ordering::Relaxed) {
            return Err(WebSocketHandlerError::TooManyIdentify);
        }

        let (id, secret) = split_token(token.as_str())?;
        verify_token(id, secret).await?;
        let bigdecimal_user_id = u128_to_bigdecimal!(id);

        let guilds = None;

        let user = {
            let res = sqlx::query!("SELECT * FROM users WHERE id = $1", bigdecimal_user_id)
                .fetch_one(db)
                .await?;
            ferrischat_common::types::User {
                id,
                name: res.name,
                avatar: res.avatar,
                guilds,
                flags: UserFlags::from_bits_truncate(res.flags),
                discriminator: res.discriminator,
                pronouns: res
                    .pronouns
                    .and_then(ferrischat_common::types::Pronouns::from_i16),
            }
        };

        inter_tx
            .send(WsOutboundEvent::IdentifyAccepted { user })
            .await?;

        uid_conn_map.insert(conn_id, id);

        Ok(())
    }
}
