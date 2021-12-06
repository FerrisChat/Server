use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use axum::Json;
use ferrischat_common::request_json::InviteCreateJson;
use ferrischat_common::types::{ErrorJson, Invite};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_snowflake_generator::FERRIS_EPOCH;
use sqlx::types::time::OffsetDateTime;

/// POST `/api/v0/guilds/{guild_id}/invites`
pub async fn create_invite(
    auth: crate::Authorization,
    Path(guild_id): Path<u128>,
    Json(InviteCreateJson { max_age, max_uses }): Json<InviteCreateJson>,
) -> Result<crate::Json<Invite>, WebServerError> {
    let db = get_db_or_fail!();

    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let owner_id = auth.0;
    let bigint_owner_id = u128_to_bigdecimal!(owner_id);

    if sqlx::query!(
        "SELECT user_id FROM members WHERE user_id = $1 AND guild_id = $2",
        bigint_owner_id,
        bigint_guild_id
    )
    .fetch_optional(db)
    .await?
    .is_none()
    {
        return Err(ErrorJson::new_403("you are not a member of this guild".to_string()).into());
    }

    let now = OffsetDateTime::now_utc().unix_timestamp()
        - (i64::try_from(FERRIS_EPOCH).expect("failed to cast the Ferris Epoch to i64"));
    let resp = sqlx::query!(
        "INSERT INTO invites VALUES ((SELECT array_to_string( \
            ARRAY(SELECT substr( \
                'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789', \
                ((random()*(36-1)+1)::integer),1) FROM generate_series(1,10)),'') \
            ), $1, $2, $3, $4, $5, $6) RETURNING code",
        bigint_owner_id,
        bigint_guild_id,
        now,
        0,
        max_uses,
        max_age
    )
    .fetch_one(db)
    .await?;

    let invite_obj = Invite {
        code: resp.code,
        owner_id,
        guild_id,
        created_at: now,
        uses: 0,
        max_uses,
        max_age,
    };

    let event = WsOutboundEvent::InviteCreate {
        invite: invite_obj.clone(),
    };

    fire_event(format!("invite_{}", guild_id), &event).await?;
    Ok(crate::Json {
        obj: invite_obj,
        code: 201,
    })
}
