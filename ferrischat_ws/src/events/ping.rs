use crate::error_handling::WsEventHandlerError;
use ferrischat_common::ws::WsOutboundEvent;
use tokio::sync::mpsc::Sender;

pub async fn handle_ping_rx<'a>(
    inter_tx: &Sender<WsOutboundEvent>,
) -> Result<(), WsEventHandlerError<'a>> {
    if inter_tx.send(WsOutboundEvent::Pong).await.is_err() {
        Err(WsEventHandlerError::Sender)
    } else {
        Ok(())
    }
}
