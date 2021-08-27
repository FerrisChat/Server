use actix_web::{web::Query, HttpRequest, HttpResponse, Responder};

use ferrischat_common::request_json::GetMessageHistoryParams;
use ferrischat_common::types::{InternalServerErrorJson, Message, MessageHistory};

/// GET /api/v0/channels/{channel_id}/messages
pub async fn get_message_history(
    req: HttpRequest,
    _: crate::Authorization,
    params: Query<GetMessageHistoryParams>,
) -> impl Responder {
    let channel_id = get_item_id!(req, "channel_Id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let db = get_db_or_fail!();

    let mut limit = params.limit.unwrap_or(100);

    if limit >= 18446744073709551616 {
        limit = None;
    }

    let messages = {
        let resp = sqlx::query!("SELECT * FROM messages WHERE message_id = $1 LIMIT $2", bigint_channel_id, limit)
            .fetch_all(db)
            .await;
        Some(match resp {
            Ok(resp) => resp
                .iter()
                .filter_map(|x| {
                    Some(Message {
                        id: bigdecimal_to_u128!(x.id),
                        content: x.content,
                        channel_id,
                        author_id: bigdecimal_to_u128!(x.author_id),
                    })
                })
                .collect(),
            Err(e) => {
                return HttpResponse::InternalServerError(InternalServerErrorJson {
                    reason: format!("database returned a error: {}", e),
                })
        })

    }

    HttpResponse::Ok().json(MessageHistory {
        messages,
    })

}