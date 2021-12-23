use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Channel, ErrorJson, Pronouns, User, UserFlags};
use ferrischat_common::ws::WsOutboundEvent;

/// POST `/v0/channels/{channel_id}/typing`
pub async fn typing_start(
    Path(channel_id): Path<u128>,
    crate::Authorization(authorized_user, is_bot): crate::Authorization,
) -> Result<http::StatusCode, WebServerError> {
    let db = get_db_or_fail!();
    let bigint_user_id = u128_to_bigdecimal!(authorized_user);
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let user = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown user with ID {}", authorized_user)))?;

    let channel = sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown channel with ID {}", channel_id)))?;

    let user_obj = User {
        id: authorized_user,
        name: user.name,
        avatar: user.avatar,
        guilds: None,
        discriminator: user.discriminator,
        flags: UserFlags::from_bits_truncate(user.flags),
        pronouns: user.pronouns.and_then(Pronouns::from_i16),
        is_bot,
    };

    let guild_id = bigdecimal_to_u128!(channel.guild_id);
    let channel_obj = Channel {
        id: channel_id,
        name: channel.name,
        guild_id,
    };

    let event = WsOutboundEvent::TypingStart {
        channel: channel_obj,
        user: user_obj,
    };

    fire_event(&event).await?;

    Ok(http::StatusCode::NO_CONTENT)
}
