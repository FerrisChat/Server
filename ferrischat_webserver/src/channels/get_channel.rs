use crate::WebServerError;
use axum::extract::Path;
use ferrischat_common::types::{Channel, NotFoundJson};
use serde::Serialize;

/// GET `/api/v0/guilds/{guild_id/channels/{channel_id}`
pub async fn get_channel(
    Path(channel_id): Path<u128>,
    _: crate::Authorization,
) -> Result<crate::Json<Channel>, WebServerError<impl Serialize>> {
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    sqlx::query!("SELECT * FROM channels WHERE id = $1", bigint_channel_id)
        .fetch_optional(get_db_or_fail!())
        .await
        .map_err(|e| WebServerError::Database(e))?
        .ok_or_else(|| {
            (
                404,
                NotFoundJson {
                    message: format!("Unknown channel with ID {}", channel_id),
                },
            )
                .into()
        })?
        .map(|c| crate::Json {
            obj: Channel {
                id: channel_id,
                name: c.name,
                guild_id: bigdecimal_to_u128!(c.guild_id),
            },
            code: 200,
        })
}
