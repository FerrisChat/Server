use crate::ws::{fire_event, WsEventError};
use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::MessageCreateJson;
use ferrischat_common::types::{BadRequestJson, InternalServerErrorJson, Message, ModelType};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/{guild_id}/channels/{channel_id}/messages
pub async fn create_message(
    auth: crate::Authorization,
    req: HttpRequest,
    json: Json<MessageCreateJson>,
) -> impl Responder {
    let MessageCreateJson { content, nonce } = json.0;

    if content.len() > 10240 {
        return HttpResponse::BadRequest().json(BadRequestJson {
            reason: "message content size must be fewer than 10,240 bytes".to_string(),
            location: None,
        });
    }

    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let node_id = get_node_id!();
    let message_id = generate_snowflake::<0>(ModelType::Message as u8, node_id);
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let author_id = auth.0;
    let bigint_author_id = u128_to_bigdecimal!(author_id);

    let db = get_db_or_fail!();

    let guild_id = {
        let resp = sqlx::query!(
            "SELECT guild_id FROM channels WHERE id = $1",
            bigint_channel_id
        )
        .fetch_one(db)
        .await;

        match resp {
            Ok(r) => bigdecimal_to_u128!(r.guild_id),
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned a error: {}", e),
                })
            }
        }
    };

    let resp = sqlx::query!(
        "INSERT INTO messages VALUES ($1, $2, $3, $4)",
        bigint_message_id,
        content,
        bigint_channel_id,
        bigint_author_id
    )
    .execute(db)
    .await;
    if let Err(e) = resp {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        });
    }

    let msg_obj = Message {
        id: message_id,
        content: Some(content),
        channel_id,
        author_id,
        author: None,
        edited_at: None,
        embeds: vec![],
        nonce: nonce,
    };

    let event = WsOutboundEvent::MessageCreate {
        message: msg_obj.clone(),
    };

    if let Err(e) = fire_event(format!("message_{}_{}", channel_id, guild_id), &event).await {
        let reason = match e {
            WsEventError::MissingRedis => "Redis pool missing".to_string(),
            WsEventError::RedisError(e) => format!("Redis returned an error: {}", e),
            WsEventError::JsonError(e) => {
                format!("Failed to serialize message to JSON format: {}", e)
            }
        };
        return HttpResponse::InternalServerError().json(InternalServerErrorJson { reason });
    }

    HttpResponse::Created().json(msg_obj)
}
