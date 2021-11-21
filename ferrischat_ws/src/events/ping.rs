use crate::error_handling::WsEventHandlerError;
use crate::TxRxComm;
use ferrischat_common::ws::WsOutboundEvent;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub async fn handle_ping_rx<'a>(
    inter_tx: &Sender<TxRxComm>,
) -> Result<(), WsEventHandlerError<'a>> {
    if inter_tx
        .send(TxRxComm::Text(
            match simd_json::serde::to_string(&WsOutboundEvent::Pong) {
                Ok(v) => v,
                Err(e) => {
                    return Err(WsEventHandlerError::CloseFrame(CloseFrame {
                        code: CloseCode::from(5001),
                        reason: format!("JSON serialization error: {}", e).into(),
                    }));
                }
            },
        ))
        .await
        .is_err()
    {
        Err(WsEventHandlerError::Sender)
    } else {
        Ok(())
    }
}
