use crate::ws::{fire_event, WsEventError};
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{InternalServerErrorJson, Member, NotFoundJson};
use ferrischat_common::ws::WsOutboundEvent;
use serde::Serialize;

/// DELETE `/api/v0/guilds/{guild_id}/members/{member_id}`
pub async fn delete_member(
    Path((guild_id, member_id)): Path<(u128, u128)>,
    _: crate::Authorization,
) -> Result<crate::Json<impl Serialize>, WebServerError<impl Serialize>> {
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let bigint_member_id = u128_to_bigdecimal!(member_id);

    let db = get_db_or_fail!();

    let owner_id_matches: bool = sqlx::query!(
        r#"SELECT EXISTS(SELECT owner_id FROM guilds WHERE id = $1 AND owner_id = $2) AS "exists!""#
    )
    .fetch_one(db)
    .await?
    .exists;
    if owner_id_matches {
        return Err((
            409,
            ferrischat_common::types::Json {
                message: "the guild owner cannot be removed from a guild".to_string(),
            },
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
            NotFoundJson {
                message: format!("Unknown member with ID {} in {}", member_id, guild_id),
            },
        )
    })?;

    let event = WsOutboundEvent::MemberDelete { member: member_obj };

    fire_event(format!("member_{}", guild_id), &event).await?;
    HttpResponse::NoContent().finish()
}
