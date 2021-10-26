use crate::ws::{fire_event, WsEventError};

use ferrischat_common::ws::WsOutboundEvent;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{Channel, InternalServerErrorJson, NotFoundJson};
use sqlx::Error;

/// DELETE /api/v0/guilds/{guild_id/channels/{channel_id}
pub async fn delete_channel(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = get_item_id!(req, "channel_id");
    let guild_id = get_item_id!(req, "guild_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let resp = sqlx::query!(
        "DELETE FROM channels WHERE id = $1 AND guild_id = $2 RETURNING *",
        bigint_channel_id,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await;

    let channel_obj = match resp {
        Ok(resp) => match resp {
            Some(channel) => Channel {
                id: bigdecimal_to_u128!(channel.id),
                guild_id: bigdecimal_to_u128!(channel.guild_id),
                name: channel.name,
            },
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: "Channel not found".to_string(),
                })
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e).to_string(),
            })
        }
    };

    let event = WsOutboundEvent::ChannelDelete {
        channel: channel_obj.clone(),
    };

    if let Err(e) = fire_event(format!("channel_{}_{}", channel_id, guild_id), &event).await {
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
