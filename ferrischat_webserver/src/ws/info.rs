use ferrischat_common::types::WsConnectionInfo;

#[allow(clippy::unused_async)]
/// GET /api/v0/ws/info
pub async fn ws_info() -> crate::Json<WsConnectionInfo> {
    crate::Json {
        obj: WsConnectionInfo {
            url: "wss://ws.api.ferris.chat".to_string(),
        },
        code: 200,
    }
}
