use actix_web::{HttpResponse, Responder};
use ferrischat_common::types::WsConnectionInfo;

/// GET /api/v0/ws/info
pub async fn ws_info(_: crate::Authorization) -> impl Responder {
    HttpResponse::Ok().json(WsConnectionInfo {
        url: "wss://api.ferris.chat:8081".to_string(),
    })
}
