use crate::{get_pool, Auth, Error, PostgresU128, PromoteErr, Response, RouteResult};
use common::{
    http::{CreateChannelPayload, HybridSnowflake},
    models::{GuildChannel, GuildChannelType, ModelType, PermissionOverwrite, Permissions},
    CastSnowflakes, Snowflake,
};

use crate::checks::assert_permissions;
use axum::{
    extract::{Json, Path},
    headers::HeaderMap,
    http::StatusCode,
    Router,
};
use ferrischat_snowflake_generator::generate_snowflake;

fn validate_channel_name(name: impl AsRef<str>) -> Result<(), Response<Error>> {
    if !(1..=32).contains(&name.as_ref().trim().chars().count()) {
        return Err(Response(
            StatusCode::BAD_REQUEST,
            Error::ValidationError {
                field: "name",
                message: "Channel name must be between 1 and 32 characters".to_string(),
            },
        ));
    }

    Ok(())
}

/// POST /guilds/:guild_id/channels
#[allow(
    clippy::too_many_lines,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
pub async fn create_channel(
    Auth(user_id, _): Auth,
    Path(guild_id): Path<u128>,
    Json(payload): Json<CreateChannelPayload<HybridSnowflake>>,
    headers: HeaderMap,
) -> RouteResult<GuildChannel> {
    assert_permissions(guild_id, user_id, None, Permissions::MANAGE_CHANNELS)
        .await
        .promote(&headers)?;

    let db = get_pool();
    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, 0);

    let inner = payload.inner();
    validate_channel_name(&inner.name).promote(&headers)?;

    let (topic, icon, user_limit) = match payload.clone() {
        CreateChannelPayload::Text { topic, icon, .. }
        | CreateChannelPayload::Announcement { topic, icon, .. } => (topic, icon, None),
        CreateChannelPayload::Voice {
            user_limit, icon, ..
        } => (None, icon, user_limit),
        CreateChannelPayload::Category { .. } => Default::default(),
    };

    let kind = payload.kind();
    let postgres_parent_id = inner
        .parent_id
        .as_ref()
        .map(|id| PostgresU128::new(id.to_u128()));
    let position = if let Some(position) = inner.position {
        position
    } else {
        match kind {
            GuildChannelType::Category => {
                sqlx::query!(
                    r#"SELECT
                        COALESCE(MAX(position) + 1, 0) AS "position!"
                    FROM
                        channels
                    WHERE
                        guild_id = $1
                    AND
                        type = 'category'
                    AND
                        (parent_id = $2 OR parent_id IS NULL AND $2 IS NULL)
                    "#,
                    PostgresU128::new(guild_id) as _,
                    postgres_parent_id as _,
                )
                .fetch_one(db)
                .await
                .promote(&headers)?
                .position as u16
            }
            _ => {
                sqlx::query!(
                    r#"SELECT
                        COALESCE(MAX(position) + 1, 0) AS "position!"
                    FROM
                        channels
                    WHERE
                        guild_id = $1
                    AND
                        (parent_id = $2 OR parent_id IS NULL AND $2 IS NULL)
                    "#,
                    PostgresU128::new(guild_id) as _,
                    postgres_parent_id as _,
                )
                .fetch_one(db)
                .await
                .promote(&headers)?
                .position as u16
            }
        }
    };

    let mut transaction = db.begin().await.promote(&headers)?;

    sqlx::query!(
        "INSERT INTO channels
            (id, guild_id, type, name, position, parent_id, topic, icon, user_limit)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ",
        PostgresU128::new(channel_id) as _,
        PostgresU128::new(guild_id) as _,
        kind.to_string(),
        inner.name,
        position as i16,
        postgres_parent_id as _,
        topic,
        icon,
        user_limit.map(|limit| limit as i16),
    )
    .execute(&mut transaction)
    .await
    .promote(&headers)?;

    let overwrites = if let Some(ref overwrites) = inner.overwrites {
        let (targets, (allow, deny)) = overwrites
            .iter()
            .map(|o| {
                (
                    PostgresU128::new(o.id.to_u128()),
                    (o.permissions.allow.bits(), o.permissions.deny.bits()),
                )
            })
            .unzip::<_, _, Vec<_>, (Vec<_>, Vec<_>)>();

        sqlx::query(
            "INSERT INTO channel_overwrites
            SELECT $1, $2, out.*
            FROM UNNEST($3, $4, $5)
            AS out(target_id, allow, deny)",
        )
        .bind(PostgresU128::new(guild_id))
        .bind(PostgresU128::new(channel_id))
        .bind(targets)
        .bind(allow)
        .bind(deny)
        .execute(&mut transaction)
        .await
        .promote(&headers)?;

        // jetbrains rust plugin cannot properly resolve the trait
        <Vec<PermissionOverwrite<_>> as CastSnowflakes>::into_u128_ids(overwrites.clone())
    } else {
        Vec::new()
    };

    transaction.commit().await.promote(&headers)?;

    Response::created(GuildChannel {
        id: channel_id,
        guild_id,
        kind,
        name: inner.name.clone(),
        position,
        overwrites,
        parent_id: inner.parent_id.as_ref().map(Snowflake::to_u128),
        topic,
        icon,
        slowmode: None,
        locked: None,
        nsfw: None,
        last_message_id: None,
        user_limit,
    })
    .promote_ok(&headers)
}

#[must_use]
pub fn router() -> Router {
    Router::new()
}
