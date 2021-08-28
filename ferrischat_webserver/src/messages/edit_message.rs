use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::MessageUpdateJson;
use ferrischat_common::types::{InternalServerErrorJson, Message, NotFoundJson};

use num_traits::ToPrimitive;

pub async fn edit_message(
    req: HttpRequest,
    message_info: Json<MessageUpdateJson>,
    auth: crate::Authorization,
) -> impl Responder {
    let channel_id = get_item_id!(req, "channel_id");
    let message_id = get_item_id!(req, "message_id");

    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let db = get_db_or_fail!();
    {
        let resp = sqlx::query!(
            "SELECT author_id FROM messages WHERE channel_id = $1 AND id = $2",
            bigint_channel_id,
            bigint_message_id
        )
        .fetch_optional(db)
        .await;

        match resp {
            Ok(resp) => match resp {
                Some(resp) => {
                    let author_id = bigdecimal_to_u128!(resp.author_id);
                    if author_id != auth.0 {
                        return HttpResponse::Forbidden().finsh();
                    }
                }
                None => {
                    return HttpResponse::NotFound().json(NotFoundJson {
                        message: "Message not found".to_string(),
                    })
                }
            },
            Err(err) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    message: format!("DB returned an error: {}", err),
                })
            }
        }
    }

    let MessageUpdateJson { content } = message_info.0;
    let resp = sqlx::query!("UPDATE messages SET content = $3 AND edited_at = CURRENT_TIMESTAMP() WHERE message_id = $1 AND id = $2 RETURNING *", bigint_message_id, bigint_channel_id, content)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(resp) => HttpResponse::Ok().json(Message {
                id: message
                    .id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128(),
                channel_id: message
                    .bigint_channel_id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128(),
                author_id: message
                    .author_id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128(),
                content: message.content,
                edited_at: message.edited_at,
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Message not found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            message: format!("DB returned an error: {}", e),
        }),
    }
}
