use crate::ws::{fire_event, WsEventError};
use ferrischat_common::ws::WsOutboundEvent;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, Member, NotFoundJson};

/// DELETE /api/v0/guilds/{guild_id}/members/{member_id}
pub async fn delete_member(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let guild_id = {
        let raw = get_item_id!(req, "guild_id");
        u128_to_bigdecimal!(raw)
    };
    let member_id = {
        let raw = get_item_id!(req, "member_id");
        u128_to_bigdecimal!(raw)
    };

    let db = get_db_or_fail!();

    let resp = sqlx::query!(
        "DELETE FROM members WHERE user_id = $1 AND guild_id = $2 RETURNING *",
        member_id,
        guild_id
    )
    .fetch_optional(db)
    .await;

    let member_obj = match resp {
        Ok(r) => match r {
            Some(_) => Member {
                user_id: Some(bigdecimal_to_u128!(member_id)),
                user: None,
                guild_id: Some(bigdecimal_to_u128!(guild_id)),
                guild: None,
            },
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: format!("Unknown member with id {}", member_id),
                })
            }
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("Database responded with an error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };

    let event = WsOutboundEvent::MemberDelete {
        member: member_obj.clone(),
    };

    if let Err(e) = fire_event(format!("member_{}", guild_id), &event).await {
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

    HttpResponse::NoContent().finish()
}
