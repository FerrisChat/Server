use crate::ws::{fire_event, WsEventError};

use ferrischat_common::ws::WsOutboundEvent;

use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::ChannelUpdateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, NotFoundJson};

pub async fn edit_channel(
    req: HttpRequest,
    channel_info: Json<ChannelUpdateJson>,
    _: crate::Authorization,
) -> impl Responder {
    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let db = get_db_or_fail!();

    let ChannelUpdateJson { name } = channel_info.0;

    let old_channel_obj = {
        let resp = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
            .fetch_optional(db)
            .await;

        match resp {
            Ok(resp) => match resp {
                Some(channel) => Channel {
                    id: bigdecimal_to_u128!(channel.id),
                    name: channel.name,
                    guild_id: bigdecimal_to_u128!(channel.guild_id),
                },
                None => {
                    return HttpResponse::NotFound().json(NotFoundJson {
                        message: format!("Unknown channel with id {}", channel_id),
                    })
                }
            },
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                    is_bug: false,
                    link: None,
                })
            }
        }
    };

    let resp = sqlx::query!(
        "UPDATE channels SET name = $1 WHERE id = $2 RETURNING *",
        name,
        bigint_channel_id
    )
    .fetch_optional(db)
    .await;

    let new_channel_obj = match resp {
        Ok(resp) => match resp {
            Some(channel) => Channel {
                id: bigdecimal_to_u128!(channel.id),
                name: channel.name,
                guild_id: bigdecimal_to_u128!(channel.guild_id),
            },
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: format!("Unknown channel with id {}", channel_id),
                })
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };

    let guild_id = new_channel_obj.guild_id;

    let event = WsOutboundEvent::ChannelUpdate {
        old: old_channel_obj,
        new: new_channel_obj.clone(),
    };

    if let Err(e) = fire_event(format!("channel_{}_{}", channel_id, guild_id), &event).await {
        let reason = match e {
            WsEventError::MissingRedis => "Redis pool missing".to_string(),
            WsEventError::RedisError(e) => format!("Redis returned an error: {}", e),
            WsEventError::JsonError(e) => {
                format!("Failed to serialize message to JSON format: {}", e)
            }
        };
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason,
            is_bug: true,
            link: Option::from(
                "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                    .to_string()),
        });
    }

    HttpResponse::Ok().json(new_channel_obj)
}
