use crate::{
    get_pool, ratelimit, Auth, Error, HeaderAwareResult, PostgresU128, PromoteErr, Response,
    RouteResult,
};
use common::{
    http::{CreateChannelPayload, EditChannelPayload, HybridSnowflake},
    models::{
        Channel, ChannelType, DMChannel, GuildChannel, GuildChannelType, ModelType,
        PermissionOverwrite, PermissionPair, Permissions,
    },
    CastSnowflakes, Snowflake,
};

use crate::checks::{assert_member, assert_permissions};
use axum::{
    extract::{Json, Path},
    handler::Handler,
    headers::HeaderMap,
    http::StatusCode,
    routing::get,
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

/// GET /channels/:id
#[allow(clippy::too_many_lines, clippy::cast_sign_loss)]
pub async fn get_channel(
    Auth(user_id, _): Auth,
    Path(channel_id): Path<u128>,
    headers: HeaderMap,
) -> RouteResult<Channel> {
    struct QueryResponse {
        guild_id: Option<PostgresU128>,
        kind: String,
        name: Option<String>,
        position: Option<i16>,
        parent_id: Option<PostgresU128>,
        topic: Option<String>,
        icon: Option<String>,
        slowmode: Option<i32>,
        nsfw: Option<bool>,
        locked: Option<bool>,
        user_limit: Option<i16>,
        owner_id: Option<PostgresU128>,
    }

    struct OverwriteResponse {
        target_id: PostgresU128,
        allow: i64,
        deny: i64,
    }

    struct UserIdResponse {
        user_id: PostgresU128,
    }

    let db = get_pool();

    let channel: QueryResponse = sqlx::query_as!(
        QueryResponse,
        r#"SELECT
            guild_id AS "guild_id: PostgresU128",
            type AS kind,
            name,
            position,
            parent_id AS "parent_id: PostgresU128",
            topic,
            icon,
            slowmode,
            locked,
            nsfw,
            user_limit,
            owner_id AS "owner_id: PostgresU128"
        FROM
            channels
        WHERE
            id = $1
        "#,
        PostgresU128::new(channel_id) as _,
    )
    .fetch_optional(db)
    .await
    .promote(&headers)?
    .ok_or_else(|| {
        Response::not_found(
            "channel",
            format!("Channel with an ID of {} not found", channel_id),
        )
    })
    .promote(&headers)?;

    let kind = ChannelType::from(channel.kind);

    Response::ok(match kind {
        ChannelType::Guild(kind) => {
            let guild_id = channel.guild_id.unwrap();
            assert_member(guild_id.to_u128(), user_id)
                .await
                .promote(&headers)?;

            let overwrites: Vec<OverwriteResponse> = sqlx::query_as!(
                OverwriteResponse,
                r#"SELECT
                    target_id AS "target_id: PostgresU128",
                    allow,
                    deny
                FROM
                    channel_overwrites
                WHERE
                    guild_id = $1
                AND
                    channel_id = $2
                "#,
                guild_id as _,
                PostgresU128::new(channel_id) as _,
            )
            .fetch_all(db)
            .await
            .promote(&headers)?;

            Channel::Guild(GuildChannel {
                id: channel_id,
                guild_id: guild_id.to_u128(),
                kind,
                name: channel.name.unwrap(),
                position: channel.position.unwrap() as u16,
                overwrites: overwrites
                    .into_iter()
                    .map(|o| PermissionOverwrite {
                        id: o.target_id.to_u128(),
                        permissions: PermissionPair {
                            allow: Permissions::from_bits_truncate(o.allow),
                            deny: Permissions::from_bits_truncate(o.deny),
                        },
                    })
                    .collect(),
                parent_id: channel.parent_id.map(|p| p.to_u128()),
                topic: channel.topic,
                icon: channel.icon,
                slowmode: channel.slowmode.map(|n| n as u32),
                locked: channel.locked,
                nsfw: channel.nsfw,
                last_message_id: None, // TODO
                user_limit: channel.user_limit.map(|n| n as u16),
            })
        }
        ChannelType::DM(kind) => {
            let recipients: Vec<u128> = sqlx::query_as!(
                UserIdResponse,
                r#"SELECT
                    user_id AS "user_id: PostgresU128"
                FROM
                    channel_recipients
                WHERE
                    channel_id = $1
                "#,
                PostgresU128::new(channel_id) as _,
            )
            .fetch_all(db)
            .await
            .promote(&headers)?
            .into_iter()
            .map(|r| r.user_id.to_u128())
            .collect();

            Channel::DM(DMChannel {
                id: channel_id,
                kind,
                last_message_id: None, // TODO
                name: channel.name,
                topic: channel.topic,
                icon: channel.icon,
                owner_id: channel.owner_id.map(|o| o.to_u128()),
                recipient_ids: recipients,
            })
        }
    })
    .promote_ok(&headers)
}

async fn get_channel_info(
    channel_id: u128,
    db: &sqlx::Pool<sqlx::postgres::Postgres>,
) -> Result<(Option<u128>, Option<u128>, ChannelType), Response<Error>> {
    struct QueryResponse {
        guild_id: Option<PostgresU128>,
        owner_id: Option<PostgresU128>,
        kind: String,
    }

    let channel: QueryResponse = sqlx::query_as!(
        QueryResponse,
        r#"SELECT
            guild_id AS "guild_id: PostgresU128",
            owner_id AS "owner_id: PostgresU128",
            type AS kind
        FROM
            channels
        WHERE
            id = $1
        "#,
        PostgresU128::new(channel_id) as _,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| {
        Response::not_found(
            "channel",
            format!("Channel with an ID of {} not found", channel_id),
        )
    })?;

    Ok((
        channel.guild_id.map(|g| g.to_u128()),
        channel.owner_id.map(|o| o.to_u128()),
        ChannelType::from(channel.kind),
    ))
}

/// DELETE /channels/:id
pub async fn delete_channel(
    Auth(user_id, _): Auth,
    Path(channel_id): Path<u128>,
    headers: HeaderMap,
) -> HeaderAwareResult<StatusCode> {
    let db = get_pool();
    let (guild_id, owner_id, kind) = get_channel_info(channel_id, db).await.promote(&headers)?;

    match kind {
        ChannelType::Guild(_) => {
            assert_permissions(
                guild_id.unwrap(),
                user_id,
                Some(channel_id),
                Permissions::MANAGE_CHANNELS,
            )
            .await
            .promote(&headers)?;
        }
        ChannelType::DM(_) => {
            if owner_id.is_some_and(|o| o.to_u128() != user_id) {
                return Response(
                    StatusCode::FORBIDDEN,
                    Error::NotOwner {
                        id: channel_id,
                        message: "You must be the owner of the group chat to delete it.",
                    },
                )
                .promote_err(&headers);
            }
        }
    }

    sqlx::query!(
        "DELETE FROM channels WHERE id = $1",
        PostgresU128::new(channel_id) as _
    )
    .execute(db)
    .await
    .promote(&headers)?;

    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /channels/:id
#[allow(clippy::cast_possible_wrap)]
pub async fn edit_channel(
    auth @ Auth(user_id, _): Auth,
    path @ Path(channel_id): Path<u128>,
    Json(EditChannelPayload {
        name,
        icon,
        topic,
        user_limit,
    }): Json<EditChannelPayload>,
    headers: HeaderMap,
) -> RouteResult<Channel> {
    let db = get_pool();
    let (guild_id, _, kind) = get_channel_info(channel_id, db).await.promote(&headers)?;

    match kind {
        ChannelType::Guild(_) => {
            assert_permissions(
                guild_id.unwrap(),
                user_id,
                Some(channel_id),
                Permissions::MODIFY_CHANNELS,
            )
            .await
            .promote(&headers)?;
        }
        ChannelType::DM(_) => {}
    }

    let mut transaction = db.begin().await.promote(&headers)?;

    macro_rules! update {
        ($query:literal, $field:ident, $map_fn:expr $(,)?) => {{
            if !$field.is_absent() {
                let value = $field.into_option().map($map_fn);

                sqlx::query!($query, value, PostgresU128::new(channel_id) as _)
                    .execute(&mut transaction)
                    .await
                    .promote(&headers)?;
            }
        }};
        ($query:literal, $field:ident $(,)?) => {{
            update!($query, $field, |v| v);
        }};
    }

    if let Some(name) = name {
        validate_channel_name(&name).promote(&headers)?;

        sqlx::query!(
            "UPDATE channels SET name = $1 WHERE id = $2",
            name,
            PostgresU128::new(channel_id) as _,
        )
        .execute(&mut transaction)
        .await
        .promote(&headers)?;
    }

    update!("UPDATE channels SET icon = $1 WHERE id = $2", icon);
    update!("UPDATE channels SET topic = $1 WHERE id = $2", topic);
    update!(
        "UPDATE channels SET user_limit = $1 WHERE id = $2",
        user_limit,
        |v| v as i16,
    );
    transaction.commit().await.promote(&headers)?;

    get_channel(auth, path, headers).await
}

#[must_use]
pub fn router() -> Router {
    Router::new().route(
        "/channels/:id",
        get(get_channel.layer(ratelimit!(4, 5)))
            .patch(edit_channel.layer(ratelimit!(4, 5)))
            .delete(delete_channel.layer(ratelimit!(4, 8))),
    )
}
