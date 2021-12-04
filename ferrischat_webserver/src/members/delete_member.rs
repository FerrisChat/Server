use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Member, ErrorJson};
use ferrischat_common::ws::WsOutboundEvent;
use serde::Serialize;

/// DELETE `/api/v0/guilds/{guild_id}/members/{member_id}`
pub async fn delete_member(
    Path((guild_id, member_id)): Path<(u128, u128)>,
    _: crate::Authorization,
) -> Result<http::StatusCode, WebServerError> {
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let bigint_member_id = u128_to_bigdecimal!(member_id);

    let db = get_db_or_fail!();

    let owner_id = sqlx::query!("SELECT owner_id FROM guilds WHERE id = $1", bigint_guild_id)
        .fetch_one(db)
        .await?
        .owner_id;
    if owner_id == bigint_member_id {
        return Err((
            409,
            ErrorJson::new_409(
                "the guild owner cannot be removed from a guild".to_string(),
            ),
        )
            .into());
    }

    let member_obj = sqlx::query!(
        "DELETE FROM members WHERE user_id = $1 AND guild_id = $2 RETURNING *",
        bigint_member_id,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await?
    .map(|_| Member {
        user_id: Some(member_id),
        user: None,
        guild_id: Some(guild_id),
        guild: None,
    })
    .ok_or_else(|| {
        (
            404,
            ErrorJson::new_404(
                format!("Unknown member with ID {} in {}", member_id, guild_id),
            ),
        )
            .into()
    })?;

    let event = WsOutboundEvent::MemberDelete { member: member_obj };

    fire_event(format!("member_{}", guild_id), &event).await?;
    Ok(http::StatusCode::NO_CONTENT)
}
