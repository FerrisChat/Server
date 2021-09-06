use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, Message, NotFoundJson};

/// GET /api/v0/guilds/{guild_id}/channels/{channel_id}/messages/{message_id}
pub async fn get_message(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let message_id = get_item_id!(req, "message_id");
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let db = get_db_or_fail!();

    let resp = sqlx::query!(
        "SELECT * FROM messages WHERE id = $1 AND channel_id = $2",
        bigint_message_id,
        bigint_channel_id,
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(r) => match r {
            Some(m) => HttpResponse::Ok().json(Message {
                id: message_id,
                content: m.content,
                channel_id,
                author_id: bigdecimal_to_u128!(m.author_id),
                edited_at: m.edited_at,
                embeds: vec![],
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "message not found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
