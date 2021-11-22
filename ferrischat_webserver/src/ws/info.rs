use actix_web::{HttpResponse, Responder};
use ferrischat_common::types::WsConnectionInfo;

#[allow(clippy::unused_async)]
/// GET /api/v0/ws/info
pub async fn ws_info(_: crate::Authorization) -> impl Responder {
    HttpResponse::Ok().json(WsConnectionInfo {
        url: "wss://ws.api.ferris.chat".to_string(),
    })
}
