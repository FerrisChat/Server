use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use axum::Json;
use ferrischat_common::perms::Permissions;
use ferrischat_common::request_json::RoleUpdateJson;
use ferrischat_common::types::{ErrorJson, Role};
use ferrischat_common::ws::WsOutboundEvent;

pub async fn edit_role(
    Path((guild_id, role_id)): Path<(u128, u128)>,
    Json(RoleUpdateJson {
        name,
        color,
        position,
        permissions,
    }): Json<RoleUpdateJson>,
    _: crate::Authorization,
) -> Result<crate::Json<Role>, WebServerError> {
    let bigint_role_id = u128_to_bigdecimal!(role_id);

    let db = get_db_or_fail!();

    let role = sqlx::query!("SELECT * FROM roles WHERE id = $1", bigint_role_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown role with ID {}", role_id)))?;
    let old_role_obj = Role {
        id: bigdecimal_to_u128!(role.id),
        name: role.name,
        color: role.color,
        position: role.position,
        guild_id: bigdecimal_to_u128!(role.parent_guild),
        permissions: Permissions::from_bits_truncate(role.permissions),
    };

    if let Some(name) = name {
        sqlx::query!(
            "UPDATE roles SET name = $1 WHERE id = $2",
            name,
            bigint_role_id
        )
        .execute(db)
        .await?;
    };

    if let Some(color) = color {
        sqlx::query!(
            "UPDATE roles SET color = $1 WHERE id = $2",
            color,
            bigint_role_id
        )
        .execute(db)
        .await?;
    }

    if let Some(position) = position {
        sqlx::query!(
            "UPDATE roles SET position = $1 WHERE id = $2",
            position,
            bigint_role_id
        )
        .execute(db)
        .await?;
    }

    if let Some(permissions) = permissions {
        sqlx::query!(
            "UPDATE roles SET permissions = $1 WHERE id = $2",
            permissions.bits(),
            bigint_role_id
        )
        .execute(db)
        .await;
    }

    let role = sqlx::query!("SELECT * FROM roles WHERE id = $1", bigint_role_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown role with ID {}", role_id)))?;
    let new_role_obj = Role {
        id: bigdecimal_to_u128!(role.id),
        name: role.name,
        color: role.color,
        position: role.position,
        guild_id: bigdecimal_to_u128!(role.parent_guild),
        permissions: Permissions::from_bits_truncate(role.permissions),
    };

    let guild_id = new_role_obj.guild_id;

    let event = WsOutboundEvent::RoleUpdate {
        old: old_role_obj,
        new: new_role_obj.clone(),
    };

    fire_event(format!("role_{}_{}", role_id, guild_id), &event).await?;
    Ok(crate::Json {
        obj: new_role_obj,
        code: 200,
    })
}
