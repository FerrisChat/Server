use crate::{get_pool, ratelimit, Auth, Error, PostgresU128, PromoteErr, Response, RouteResult};
use common::{
    http::CreateGuildPayload,
    models::{
        Guild, GuildChannel, GuildChannelType, GuildFlags, GuildMemberCount, MaybePartialUser,
        Member, ModelType, PartialGuild, PermissionPair, Permissions, Role, RoleFlags,
    },
};

use axum::{
    extract::Json, handler::Handler, headers::HeaderMap, http::StatusCode, routing::post, Router,
};
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /guilds
#[allow(clippy::too_many_lines, clippy::cast_possible_wrap)]
pub async fn create_guild(
    Auth(user_id, _): Auth,
    Json(CreateGuildPayload {
        name,
        description,
        icon,
        banner,
        public,
    }): Json<CreateGuildPayload>,
    headers: HeaderMap,
) -> RouteResult<Guild> {
    if !(2..=100).contains(&name.chars().count()) {
        return Response(
            StatusCode::BAD_REQUEST,
            Error::ValidationError {
                field: "name",
                message: "Guild name must be between 2 and 100 characters long".to_string(),
            },
        )
        .promote_err(&headers);
    }

    if let Some(ref description) = description {
        if description.len() > 1024 {
            return Response(
                StatusCode::BAD_REQUEST,
                Error::ValidationError {
                    field: "description",
                    message: "Guild description must under 1 KB in size".to_string(),
                },
            )
            .promote_err(&headers);
        }
    }

    let db = get_pool();
    let mut transaction = db.begin().await.promote(&headers)?;

    let guild_id = generate_snowflake::<0>(ModelType::Guild as u8, 0);
    let flags = if public {
        GuildFlags::PUBLIC
    } else {
        GuildFlags::empty()
    };

    sqlx::query!(
        "INSERT INTO
            guilds (id, name, description, icon, banner, owner_id, flags)
        VALUES
            ($1, $2, $3, $4, $5, $6, $7)
        ",
        PostgresU128::new(guild_id) as _,
        name,
        description,
        icon,
        banner,
        PostgresU128::new(user_id) as _,
        flags.bits() as i32,
    )
    .execute(&mut transaction)
    .await
    .promote(&headers)?;

    let joined_at = sqlx::query!(
        "INSERT INTO members (id, guild_id) VALUES ($1, $2) RETURNING joined_at",
        PostgresU128::new(user_id) as _,
        PostgresU128::new(guild_id) as _,
    )
    .fetch_one(&mut transaction)
    .await
    .promote(&headers)?
    .joined_at;

    let role_flags = RoleFlags::DEFAULT;
    let perms = sqlx::query!(
        "INSERT INTO roles
            (id, guild_id, name, flags)
        VALUES
            ($1, $2, 'Default', $3)
        RETURNING
            allowed_permissions, denied_permissions
        ",
        PostgresU128::new(guild_id) as _,
        PostgresU128::new(guild_id) as _,
        role_flags.bits() as i32,
    )
    .fetch_one(&mut transaction)
    .await
    .promote(&headers)?;

    // the unwraps here are safe because we just inserted the role, and
    // the rows have default values
    let allowed_perms = perms.allowed_permissions.unwrap();
    let denied_perms = perms.denied_permissions.unwrap();

    sqlx::query!(
        "INSERT INTO role_data (guild_id, role_id, user_id) VALUES ($1, $1, $2)",
        PostgresU128::new(guild_id) as _,
        PostgresU128::new(user_id) as _,
    )
    .execute(&mut transaction)
    .await
    .promote(&headers)?;

    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, 0);
    sqlx::query!(
        "INSERT INTO channels VALUES (
            $1, $2, 'text', 'general', 0, null, null, null, 0, false, false
        )
        ",
        PostgresU128::new(channel_id) as _,
        PostgresU128::new(guild_id) as _,
    )
    .execute(&mut transaction)
    .await
    .promote(&headers)?;

    transaction.commit().await.promote(&headers)?;

    let partial = PartialGuild {
        id: guild_id,
        name,
        description: description.clone(),
        icon,
        banner,
        owner_id: user_id,
        flags,
        member_count: Some(GuildMemberCount {
            total: 1,
            online: None,
        }),
    };

    let role = Role {
        guild_id,
        id: guild_id,
        name: "Default".to_string(),
        color: None,
        permissions: PermissionPair {
            allow: Permissions::from_bits_truncate(allowed_perms),
            deny: Permissions::from_bits_truncate(denied_perms),
        },
        flags: role_flags,
        position: 0,
    };

    let channel = GuildChannel {
        id: channel_id,
        guild_id,
        name: "general".to_string(),
        kind: GuildChannelType::Text,
        position: 0,
        overwrites: vec![],
        parent_id: None,
        icon: None,
        topic: None,
        locked: Some(false),
        nsfw: Some(false),
        slowmode: Some(0),
        last_message_id: None,
        user_limit: None,
    };

    let member = Member {
        user: MaybePartialUser::Partial { id: user_id },
        guild_id,
        nick: None,
        roles: Some(vec![guild_id]),
        joined_at,
    };

    Response::created(Guild {
        partial,
        members: Some(vec![member]),
        roles: Some(vec![role]),
        channels: Some(vec![channel]),
        vanity_url: None,
    })
    .promote_ok(&headers)
}

#[must_use]
pub fn router() -> Router {
    Router::new().route("/guilds", post(create_guild.layer(ratelimit!(2, 30))))
}
