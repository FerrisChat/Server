use crate::ws::{fire_event, WsEventError};

use ferrischat_common::ws::WsOutboundEvent;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, User, UserFlags};

/// DELETE /api/v0/guilds/{guild_id}/channels/{channel_id}/messages/{message_id}
pub async fn delete_message(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let message_id = get_item_id!(req, "message_id");
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

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

    let message = {
        let resp = sqlx::query!(
            "SELECT m.*, (SELECT u.* FROM users u WHERE id = m.author_id) AS author FROM messages m WHERE id = $1 AND channel_id = $2",
            bigint_message_id,
            bigint_channel_id,
        )
        .fetch_optional(db)
        .await;

        match resp {
            Ok(r) => match r {
                Some(message) => message,
                None => {
                    return HttpResponse::NotFound().json(NotFoundJson {
                        message: "message not found".to_string(),
                    })
                }
            },
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                })
            }
        }
    };

    let author_id = bigdecimal_to_u128!(message.author_id);

    let msg_obj = Message {
        id: message_id,
        channel_id,
        author_id: author_id.clone(),
        content: message.content,
        edited_at: message.edited_at,
        embeds: vec![],
        author: User {
            id: author_id,
            name: message.author.name,
            avatar: None,
            guilds: None,
            flags: UserFlags::from_bits_truncate(message.author.flags),
            discriminator: message.author.discriminator,
        },
        nonce: None,
    };

    let resp = sqlx::query!(
        "DELETE FROM messages WHERE id = $1 AND channel_id = $2",
        bigint_message_id,
        bigint_channel_id
    )
    .execute(db)
    .await;

    match resp {
        Ok(_) => (),
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB return an error: {}", e),
            })
        }
    }

    let event = WsOutboundEvent::MessageDelete {
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

    HttpResponse::NoContent().finish()
}
