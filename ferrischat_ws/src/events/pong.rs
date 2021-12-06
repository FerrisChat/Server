use crate::error_handling::WsEventHandlerError;
use ferrischat_common::ws::WsOutboundEvent;
use tokio::sync::mpsc::Sender;

pub async fn handle_pong_rx<'a>(
    inter_tx: &Sender<WsOutboundEvent>,
) -> Result<(), WsEventHandlerError<'a>> {
    if inter_tx.send(WsOutboundEvent::Ping).await.is_err() {
        Err(WsEventHandlerError::Sender)
    } else {
        Ok(())
    }
}
