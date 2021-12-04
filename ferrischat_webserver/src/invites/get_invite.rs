use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Invite, ErrorJson};
use serde::Serialize;

/// GET api/v0/invites/{code}
pub async fn get_invite(
    Path(code): Path<String>,
    _: crate::Authorization,
) -> Result<crate::Json<Invite>, WebServerError> {
    Ok(sqlx::query!("SELECT * FROM invites WHERE code = $1", code)
        .fetch_optional(get_db_or_fail!())
        .await?
        .map(|r| crate::Json {
            obj: Invite {
                code: code.to_string(),
                owner_id: bigdecimal_to_u128!(r.owner_id),
                guild_id: bigdecimal_to_u128!(r.guild_id),
                created_at: r.created_at,
                uses: r.uses,
                max_uses: r.max_uses,
                max_age: r.max_age,
            },
            code: 200,
        })
        .ok_or_else(|| {
            (
                404,
                ErrorJson::new_404(
                    format!("Unknown invite with code {}", code),
                ),
            )
                .into()
        })?)
}
