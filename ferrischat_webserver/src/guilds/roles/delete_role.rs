use crate::ws::{fire_event, WsEventError};
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::perms::Permissions;
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, Role};
use ferrischat_common::ws::WsOutboundEvent;

/// DELETE `/api/v0/guilds/{guild_id/roles/{role_id}`
pub async fn delete_role(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let role_id = get_item_id!(req, "role_id");
    let guild_id = get_item_id!(req, "guild_id");
    let bigint_role_id = u128_to_bigdecimal!(role_id);
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let resp = sqlx::query!(
        "DELETE FROM roles WHERE id = $1 AND parent_guild = $2 RETURNING *",
        bigint_role_id,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await;

    let role_obj = match resp {
        Ok(resp) => match resp {
            Some(role) => Role {
                id: bigdecimal_to_u128!(role.id),
                guild_id: bigdecimal_to_u128!(role.parent_guild),
                name: role.name,
                color: role.color,
                position: role.position,
                permissions: Permissions::from_bits_truncate(role.permissions),
            },
            None => {
                return HttpResponse::NotFound().json(NotFoundJson {
                    message: format!("Unknown role with id {}", role_id),
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

    let event = WsOutboundEvent::RoleDelete {
        role: role_obj.clone(),
    };

    if let Err(e) = fire_event(format!("role_{}_{}", role_id, guild_id), &event).await {
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
            link: Some(format!(
                "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+{}",
                reason.replace(' ', "+")
            )),
        });
    }

    HttpResponse::NoContent().finish()
}
