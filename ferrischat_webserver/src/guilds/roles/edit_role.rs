use crate::ws::{fire_event, WsEventError};

use ferrischat_common::ws::WsOutboundEvent;

use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::perms::Permissions;
use ferrischat_common::request_json::RoleUpdateJson;
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, Role};

pub async fn edit_role(
    req: HttpRequest,
    role_info: Json<RoleUpdateJson>,
    _: crate::Authorization,
) -> impl Responder {
    let role_id = get_item_id!(req, "role_id");
    let bigint_role_id = u128_to_bigdecimal!(role_id);

    let db = get_db_or_fail!();

    let RoleUpdateJson {
        name,
        color,
        position,
        permissions,
    } = role_info.0;

    let old_role_obj = {
        let resp = sqlx::query!("SELECT * FROM roles WHERE id = $1", bigint_role_id)
            .fetch_optional(db)
            .await;

        match resp {
            Ok(resp) => match resp {
                Some(role) => Role {
                    id: bigdecimal_to_u128!(role.id),
                    name: role.name,
                    color: role.color,
                    position: role.position,
                    guild_id: bigdecimal_to_u128!(role.parent_guild),
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
        }
    };

    if let Some(name) = name {
        if let Err(e) = sqlx::query!(
            "UPDATE roles SET name = $1 WHERE id = $2",
            name,
            bigint_role_id
        )
        .execute(db)
        .await
        {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            });
        };
    }

    if let Some(color) = color {
        if let Err(e) = sqlx::query!(
            "UPDATE roles SET color = $1 WHERE id = $2",
            color,
            bigint_role_id
        )
        .execute(db)
        .await
        {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            });
        };
    }

    if let Some(position) = position {
        if let Err(e) = sqlx::query!(
            "UPDATE roles SET position = $1 WHERE id = $2",
            position,
            bigint_role_id
        )
        .execute(db)
        .await
        {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            });
        };
    }

    if let Some(permissions) = permissions {
        if let Err(e) = sqlx::query!(
            "UPDATE roles SET permissions = $1 WHERE id = $2",
            permissions.bits(),
            bigint_role_id
        )
        .execute(db)
        .await
        {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            });
        };
    }

    let new_role_obj = {
        let resp = sqlx::query!("SELECT * FROM roles WHERE id = $1", bigint_role_id)
            .fetch_optional(db)
            .await;

        match resp {
            Ok(resp) => match resp {
                Some(role) => Role {
                    id: bigdecimal_to_u128!(role.id),
                    name: role.name,
                    color: role.color,
                    position: role.position,
                    guild_id: bigdecimal_to_u128!(role.parent_guild),
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
        }
    };

    let guild_id = new_role_obj.guild_id;

    let event = WsOutboundEvent::RoleUpdate {
        old: old_role_obj,
        new: new_role_obj.clone(),
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

    HttpResponse::Ok().json(new_role_obj)
}
