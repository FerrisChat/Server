use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Channel, NotFoundJson, Pronouns, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;
use serde::Serialize;

/// DELETE `/api/v0/channels/{channel_id}/typing`
pub async fn typing_end(
    Path(channel_id): Path<u128>,
    crate::Authorization(authorized_user): crate::Authorization,
) -> Result<http: StatusCode, WebServerError<impl Serialize>> {
    let db = get_db_or_fail!();
    let bigint_user_id = u128_to_bigdecimal!(auth.0);

    let user = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| {
            (
                404,
                NotFoundJson {
                    message: format!("Unknown user with ID {}", auth.0),
                },
            )
        })?;

    let user_obj = User {
        id: auth.0,
        username: user.username,
        avatar: None,
        guilds: None,
        discriminator: user.discriminator,
        flags: UserFlags::from_bits_truncate(user.flags),
        pronouns: user.pronouns.and_then(Pronouns::from_i16),
    };

    let event = WsOutboundEvent::TypingEnd {
        channel_id: channel_id,
        user: user_obj,
    };

    fire_event(format!("typing_{}", guild_id), event).await?;

    Ok(http::StatusCode::NO_CONTENT)
}
