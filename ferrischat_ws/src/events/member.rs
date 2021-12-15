use ferrischat_common::ws::WsOutboundEvent;
use sqlx::{Pool, Postgres};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub async fn handle_member_tx<'a>(
    db: &Pool<Postgres>,
    msg: &WsOutboundEvent,
    user_id: u128,
    guild_id: u128,
) -> Result<bool, CloseFrame<'a>> {
    // FIXME: once implemented, do a query to check the user has permissions to read messages in here
    let bigint_user_id = u128_to_bigdecimal!(user_id);
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    match msg {
        WsOutboundEvent::MemberDelete { .. } => Ok(true),
        _ => {
            match sqlx::query!(
                "SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2",
                bigint_user_id,
                bigint_guild_id,
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
        }
    }
}
