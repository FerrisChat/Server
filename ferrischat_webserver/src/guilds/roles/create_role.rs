use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use axum::Json;
use ferrischat_common::perms::Permissions;
use ferrischat_common::request_json::RoleCreateJson;
use ferrischat_common::types::{ModelType, Role};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST `/api/v0/guilds/{guild_id}/roles`
pub async fn create_role(
    _: crate::Authorization,
    role_info: Json<RoleCreateJson>,
    Path(guild_id): Path<u128>,
) -> Result<crate::Json<Role>, WebServerError> {
    let db = get_db_or_fail!();

    let RoleCreateJson {
        name,
        color,
        position,
        permissions,
    } = role_info.0;

    let name = name.unwrap_or_else(|| String::from("new role"));
    let position = position.unwrap_or(0);
    let permissions = permissions.unwrap_or_else(Permissions::empty);

    let node_id = get_node_id!();
    let role_id = generate_snowflake::<0>(ModelType::Role as u8, node_id);
    let bigint_role_id = u128_to_bigdecimal!(role_id);

    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let perms = b"".as_slice();
    sqlx::query!(
        "INSERT INTO roles VALUES ($1, $2, $3, $4, $5, $6)",
        bigint_role_id,
        name,
        color,
        position,
        perms,
        bigint_guild_id
    )
    .execute(db)
    .await?;

    let role_obj = Role {
        id: role_id,
        name,
        color,
        position,
        guild_id,
        permissions,
    };

    let event = WsOutboundEvent::RoleCreate {
        role: role_obj.clone(),
    };

    fire_event(format!("role_{}_{}", guild_id, role_id), &event).await?;

    Ok(crate::Json {
        obj: role_obj,
        code: 201,
    })
}
