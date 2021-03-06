use ferrischat_common::ws::WsOutboundEvent;
use sqlx::types::BigDecimal;
use sqlx::{Pool, Postgres};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub async fn handle_typing_tx<'a>(
    db: &Pool<Postgres>,
    msg: &WsOutboundEvent,
    bigdecimal_uid: BigDecimal,
    guild_id: u128,
) -> Result<bool, CloseFrame<'a>> {
    // FIXME: once implemented, do a query to check the user has permissions to view channel in here

    match sqlx::query!(
        "SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2",
        bigdecimal_uid,
        u128_to_bigdecimal!(guild_id)
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(_)) => Ok(true),
        Ok(None) => Ok(false),
        Err(e) => Err(CloseFrame {
            code: CloseCode::from(5000),
            reason: format!("Internal database error: {}", e).into(),
        }),
    }