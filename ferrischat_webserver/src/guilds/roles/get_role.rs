use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::perms::Permissions;
use ferrischat_common::types::{ErrorJson, Role};

/// GET `/v0/guilds/{guild_id/roles/{role_id}`
pub async fn get_role(
    Path(role_id): Path<u128>,
    _: crate::Authorization,
) -> Result<crate::Json<Role>, WebServerError> {
    let bigdecimal_role_id = u128_to_bigdecimal!(role_id);
    let r = sqlx::query!("SELECT * FROM roles where id = $1", bigdecimal_role_id)
        .fetch_optional(get_db_or_fail!())
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown role with ID {}", role_id)))?;
    Ok(crate::Json {
        obj: Role {
            id: role_id,
            name: r.name,
            color: r.color,
            position: r.position,
            guild_id: bigdecimal_to_u128!(r.parent_guild),
            permissions: Permissions::empty(),
        },
        code: 200,
    })
}
