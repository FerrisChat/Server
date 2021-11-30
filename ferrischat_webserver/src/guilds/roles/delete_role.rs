use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::perms::Permissions;
use ferrischat_common::types::{NotFoundJson, Role};
use ferrischat_common::ws::WsOutboundEvent;
use http::StatusCode;
use serde::Serialize;

/// DELETE `/api/v0/guilds/{guild_id/roles/{role_id}`
pub async fn delete_role(
    Path((guild_id, role_id)): Path<(u128, u128)>,
    _: crate::Authorization,
) -> Result<StatusCode, WebServerError<impl Serialize>> {
    let db = get_db_or_fail!();
    let bigint_role_id = u128_to_bigdecimal!(role_id);
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let role_obj: Role = sqlx::query!(
        "DELETE FROM roles WHERE id = $1 AND parent_guild = $2 RETURNING *",
        bigint_role_id,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await?
    .map(|role| Role {
        id: bigdecimal_to_u128!(role.id),
        guild_id: bigdecimal_to_u128!(role.parent_guild),
        name: role.name,
        color: role.color,
        position: role.position,
        permissions: Permissions::from_bits_truncate(role.permissions),
    })
    .ok_or_else(|| {
        (
            404,
            NotFoundJson {
                message: format!("Unknown role with ID {}", role_id),
            },
        )
    })?;

    let event = WsOutboundEvent::RoleDelete {
        role: role_obj.clone(),
    };

    fire_event(format!("role_{}_{}", role_id, guild_id), &event).await?;
    Ok(StatusCode::NO_CONTENT)
}
