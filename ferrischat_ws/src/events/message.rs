use crate::types::WsTransmit;
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_redis::redis::Msg;
use futures_util::SinkExt;
use sqlx::types::BigDecimal;
use sqlx::{Pool, Postgres};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;

pub async fn handle_message_tx<'a>(
    tx: &mut WsTransmit,
    db: &Pool<Postgres>,
    msg: Msg,
    bigdecimal_uid: BigDecimal,
    _: u128,
    guild_id: u128,
) -> Result<(), CloseFrame<'a>> {
    // FIXME: once implemented, do a query to check the user has permissions to read messages in here
    match sqlx::query!(
        "SELECT guild_id FROM members WHERE user_id = $1 AND guild_id = $2",
        bigdecimal_uid,
        u128_to_bigdecimal!(guild_id)
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(_)) => {}
        Ok(None) => return Ok(()),
        Err(e) => {
            return Err(CloseFrame {
                code: CloseCode::from(5000),
                reason: format!("Internal database error: {}", e).into(),
            })
        }
    }

    // all checks completed, fire event
    let outbound_message =
        match simd_json::serde::from_reader::<_, WsOutboundEvent>(msg.get_payload_bytes()) {
            Ok(msg) => msg,
            Err(e) => {
                return Err(CloseFrame {
                    code: CloseCode::from(5005),
                    reason: format!("Internal JSON representation decoding failed: {}", e).into(),
                })
            }
        };
    let outbound_message = match simd_json::to_string(&outbound_message) {
        Ok(msg) => msg,
        Err(e) => {
            return Err(CloseFrame {
                code: CloseCode::from(5001),
                reason: format!("JSON serialization error: {}", e).into(),
            })
        }
    };
    let _ = tx.feed(Message::Text(outbound_message)).await;
    Ok(())
}
