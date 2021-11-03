use crate::ws::{fire_event, WsEventError};

use ferrischat_common::ws::WsOutboundEvent;

use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::perms::Permissions;
use ferrischat_common::request_json::RoleCreateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, ModelType, Role};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/{guild_id}/roles
pub async fn create_role(
    _: crate::Authorization,
    role_info: Json<RoleCreateJson>,
    req: HttpRequest,
) -> impl Responder {
    let db = get_db_or_fail!();

    let RoleCreateJson {
        name,
        color,
        position,
        permissions,
    } = role_info.0;

    let name = name.unwrap_or_else(|| String::from("new role"));
    let position = position.unwrap_or(0);
    let permissions = permissions.unwrap_or(Permissions::empty());

    let node_id = get_node_id!();
    let role_id = generate_snowflake::<0>(ModelType::Role as u8, node_id);
    let bigint_role_id = u128_to_bigdecimal!(role_id);

    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let permissions = Permissions::empty();

    let resp = sqlx::query!(
        "INSERT INTO roles VALUES ($1, $2, $3, $4, $5, $6)",
        bigint_role_id,
        name,
        color,
        position,
        permissions.bits(),
        bigint_guild_id
    )
    .execute(db)
    .await;

    let role_obj = match resp {
        Ok(_) => Role {
            id: role_id,
            name,
            color,
            position,
            guild_id,
            permissions,
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned a error: {}", e),
            })
        }
    };

    let event = WsOutboundEvent::RoleCreate {
        role: role_obj.clone(),
    };

    if let Err(e) = fire_event(format!("role_{}_{}", guild_id, role_id), &event).await {
        let reason = match e {
            WsEventError::MissingRedis => "Redis pool missing".to_string(),
            WsEventError::RedisError(e) => format!("Redis returned an error: {}", e),
            WsEventError::JsonError(e) => {
                format!("Failed to serialize message to JSON format: {}", e)
            }
        };
        return HttpResponse::InternalServerError().json(InternalServerErrorJson { reason });
    }

    HttpResponse::Created().json(role_obj)
}
