use crate::ws::{fire_event, WsEventError};
use ferrischat_common::ws::WsOutboundEvent;

use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::GuildUpdateJson;
use ferrischat_common::types::{Guild, GuildFlags, InternalServerErrorJson, NotFoundJson};

pub async fn edit_guild(
    req: HttpRequest,
    guild_info: Json<GuildUpdateJson>,
    _: crate::Authorization,
) -> impl Responder {
    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let GuildUpdateJson { name } = guild_info.0;

    let db = get_db_or_fail!();

    let old_guild_obj = {
        let resp = sqlx::query!("SELECT * FROM guilds WHERE id = $1", bigint_guild_id)
            .fetch_optional(db)
            .await;

        match resp {
            Ok(resp) => match resp {
                Some(guild) => Guild {
                    id: bigdecimal_to_u128!(guild.id),
                    owner_id: bigdecimal_to_u128!(guild.owner_id),
                    name: guild.name.clone(),
                    flags: GuildFlags::none(),
                    channels: None,
                    members: None,
                },
                None => {
                    return HttpResponse::NotFound().json(NotFoundJson {
                        message: "Guild not found".to_string(),
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

    let resp = sqlx::query!(
        "UPDATE guilds SET name = $1 WHERE id = $2 RETURNING *",
        name,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await;

    let new_guild_obj = match resp {
        Ok(resp) => match resp {
            Some(guild) => Guild {
                id: bigdecimal_to_u128!(guild.id),
                owner_id: bigdecimal_to_u128!(guild.owner_id),
                name: guild.name.clone(),
                channels: None,
                flags: GuildFlags::none(),
                members: None,
            },
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: "Guild not found".to_string(),
                })
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
            })
        }
    };

    let event = WsOutboundEvent::GuildUpdate {
        old: old_guild_obj,
        new: new_guild_obj.clone(),
    };

    if let Err(e) = fire_event(format!("guild_{}", guild_id), &event).await {
        let reason = match e {
            WsEventError::MissingRedis => "Redis pool missing".to_string(),
            WsEventError::RedisError(e) => format!("Redis returned an error: {}", e),
            WsEventError::JsonError(e) => {
                format!("Failed to serialize message to JSON format: {}", e)
            }
        };
        return HttpResponse::InternalServerError().json(InternalServerErrorJson { reason });
    }

    HttpResponse::Ok().json(new_guild_obj)
}
