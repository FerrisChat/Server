use actix_web::{HttpResponse, Responder};
use ferrischat_common::types::WsConnectionInfo;

/// GET /api/v0/ws/info
pub async fn ws_info(_: crate::Authorization) -> impl Responder {
    HttpResponse::Ok().json(WsConnectionInfo {
        url: "wss://ferris.chat/api/v0/ws/connect".to_string(),
    })
}
