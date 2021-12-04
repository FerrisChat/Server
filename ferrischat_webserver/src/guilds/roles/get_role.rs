use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::perms::Permissions;
use ferrischat_common::types::{NotFoundJson, Role};
use serde::Serialize;

/// GET `/api/v0/guilds/{guild_id/roles/{role_id}`
pub async fn get_role(
    Path(role_id): Path<u128>,
    _: crate::Authorization,
) -> Result<crate::Json<Role>, WebServerError> {
    let bigint_role_id = u128_to_bigdecimal!(role_id);
    Ok(
        sqlx::query!("SELECT * FROM roles where id = $1", bigint_role_id)
            .fetch_optional(get_db_or_fail!())
            .await?
            .map(|r| crate::Json {
                obj: Role {
                    id: role_id,
                    name: r.name,
                    color: r.color,
                    position: r.position,
                    guild_id: bigdecimal_to_u128!(r.parent_guild),
                    permissions: Permissions::from_bits_truncate(r.permissions),
                },
                code: 200,
            })
            .ok_or_else(|| {
                (
                    404,
                    NotFoundJson {
                        message: format!("Unknown role with ID {}", role_id),
                    },
                )
                    .into()
            })?,
    )
}
