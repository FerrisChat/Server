use crate::{
    checks::assert_member, get_pool, ratelimit, Auth, Error, HeaderAwareResult, PostgresU128,
    PromoteErr, Response, RouteResult, StatusCode,
};
use common::{
    http::{CreateGuildPayload, DeleteGuildPayload, GetGuildQuery},
    models::{
        Guild, GuildChannel, GuildChannelType, GuildFlags, GuildMemberCount, MaybePartialUser,
        Member, ModelType, PartialGuild, PermissionPair, Permissions, Role, RoleColor, RoleFlags,
        User, UserFlags,
    },
};

use axum::{
    extract::{Json, Path, Query},
    handler::Handler,
    headers::HeaderMap,
    routing::{get, post},
    Router,
};
use common::models::PermissionOverwrite;
use ferrischat_snowflake_generator::generate_snowflake;
use itertools::Itertools;
use std::collections::HashMap;

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
            Error::<u128>::ValidationError {
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
                Error::<u128>::ValidationError {
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
            (id, guild_id, name, flags, position)
        VALUES
            ($1, $2, 'Default', $3, 0)
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
    let allowed_perms = perms.allowed_permissions;
    let denied_perms = perms.denied_permissions;

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

/// GET /guilds/:id
#[allow(
    clippy::too_many_lines,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
pub async fn get_guild(
    Auth(user_id, _): Auth,
    Path(guild_id): Path<u128>,
    Query(GetGuildQuery {
        channels,
        roles,
        members,
    }): Query<GetGuildQuery>,
    headers: HeaderMap,
) -> RouteResult<Guild> {
    struct GuildQueryResponse {
        name: String,
        description: Option<String>,
        icon: Option<String>,
        banner: Option<String>,
        owner_id: PostgresU128,
        flags: i32,
        member_count: i64,
        vanity_url: Option<String>,
    }

    struct ChannelQueryResponse {
        id: PostgresU128,
        name: String,
        kind: String,
        position: i16,
        parent_id: Option<PostgresU128>,
        icon: Option<String>,
        topic: Option<String>,
        locked: Option<bool>,
        nsfw: Option<bool>,
        slowmode: Option<i32>,
        user_limit: Option<i16>,
    }

    struct OverwriteQueryResponse {
        channel_id: PostgresU128,
        target_id: PostgresU128,
        allow: i64,
        deny: i64,
    }

    struct RoleQueryResponse {
        id: PostgresU128,
        name: String,
        color: Option<Vec<i32>>,
        gradient: bool,
        allowed_permissions: i64,
        denied_permissions: i64,
        flags: i32,
        position: i16,
    }

    struct MemberQueryResponse {
        id: PostgresU128,
        nick: Option<String>,
        joined_at: common::Timestamp,
        username: String,
        discriminator: i16,
        avatar: Option<String>,
        banner: Option<String>,
        bio: Option<String>,
        flags: i32,
    }

    struct MemberRoleQueryResponse {
        user_id: PostgresU128,
        role_id: PostgresU128,
    }

    assert_member(guild_id, user_id).await.promote(&headers)?;

    let db = get_pool();
    let guild: GuildQueryResponse = sqlx::query_as!(
        GuildQueryResponse,
        r#"SELECT
            name,
            description,
            icon,
            banner,
            owner_id AS "owner_id: PostgresU128",
            flags,
            vanity_url,
            (SELECT COUNT(*) FROM members WHERE guild_id = $1) AS "member_count!"
        FROM
            guilds
        WHERE
            id = $1
        "#,
        PostgresU128::new(guild_id) as _,
    )
    .fetch_optional(db)
    .await
    .promote(&headers)?
    .ok_or_else(|| {
        Response(
            StatusCode::NOT_FOUND,
            Error::<u128>::NotFound {
                entity: "guild",
                message: format!("Guild with ID {} not found", guild_id),
            },
        )
    })
    .promote(&headers)?;

    let partial = PartialGuild {
        id: guild_id,
        name: guild.name,
        description: guild.description,
        icon: guild.icon,
        banner: guild.banner,
        owner_id: guild.owner_id.to_u128(),
        flags: GuildFlags::from_bits_truncate(guild.flags as u32),
        member_count: Some(GuildMemberCount {
            total: guild.member_count as u32,
            online: None, // TODO
        }),
    };

    let channels = if channels {
        let channels: Vec<ChannelQueryResponse> = sqlx::query_as!(
            ChannelQueryResponse,
            r#"SELECT
                id AS "id: PostgresU128",
                name AS "name!",
                type AS kind,
                position AS "position!",
                parent_id AS "parent_id?: PostgresU128",
                icon,
                topic,
                locked,
                nsfw,
                slowmode,
                user_limit
            FROM
                channels
            WHERE
                guild_id = $1
            "#,
            PostgresU128::new(guild_id) as _,
        )
        .fetch_all(db)
        .await
        .promote(&headers)?;

        let overwrites: Vec<OverwriteQueryResponse> = sqlx::query_as!(
            OverwriteQueryResponse,
            r#"
            SELECT
                channel_id AS "channel_id: PostgresU128",
                target_id AS "target_id: PostgresU128",
                allow,
                deny
            FROM channel_overwrites
            WHERE channel_id IN (SELECT id FROM channels WHERE guild_id = $1)"#,
            PostgresU128::new(guild_id) as _,
        )
        .fetch_all(db)
        .await
        .promote(&headers)?;

        let overwrites = overwrites
            .into_iter()
            .into_group_map_by(|o| o.channel_id.to_u128());

        let mut overwrites = overwrites
            .into_iter()
            .map(|(c, o)| {
                (
                    c,
                    Some(
                        o.into_iter()
                            .map(|o| PermissionOverwrite {
                                id: o.target_id.to_u128(),
                                permissions: PermissionPair {
                                    allow: Permissions::from_bits_truncate(o.allow),
                                    deny: Permissions::from_bits_truncate(o.deny),
                                },
                            })
                            .collect::<Vec<_>>(),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

        Some(
            channels
                .into_iter()
                .map(|c| GuildChannel {
                    id: c.id.to_u128(),
                    guild_id,
                    name: c.name,
                    kind: GuildChannelType::from(c.kind),
                    position: c.position as u16,
                    overwrites: overwrites
                        .get_mut(&c.id.to_u128())
                        .unwrap_or(&mut None)
                        .take()
                        .unwrap_or_default(),
                    parent_id: c.parent_id.map(|p| p.to_u128()),
                    topic: c.topic,
                    icon: c.icon,
                    slowmode: c.slowmode.map(|s| s as u32),
                    locked: c.locked,
                    nsfw: c.nsfw,
                    last_message_id: None,
                    user_limit: c.user_limit.map(|u| u as u16),
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    let roles = if roles || members {
        let roles: Vec<RoleQueryResponse> = sqlx::query_as!(
            RoleQueryResponse,
            r#"SELECT
                id AS "id: PostgresU128",
                name,
                color,
                gradient,
                allowed_permissions,
                denied_permissions,
                flags,
                position
            FROM
                roles
            WHERE
                guild_id = $1
            "#,
            PostgresU128::new(guild_id) as _,
        )
        .fetch_all(db)
        .await
        .promote(&headers)?;

        Some(
            roles
                .into_iter()
                .map(|r| Role {
                    id: r.id.to_u128(),
                    guild_id,
                    name: r.name,
                    color: r.color.map(|c| {
                        if r.gradient {
                            RoleColor::Gradient {
                                value: c
                                    .into_iter()
                                    .map(|c| ((c >> 8) as u32, (c & 0xff) as u8))
                                    .collect(),
                            }
                        } else {
                            RoleColor::Solid { value: c[0] as u32 }
                        }
                    }),
                    permissions: PermissionPair {
                        allow: Permissions::from_bits_truncate(r.allowed_permissions),
                        deny: Permissions::from_bits_truncate(r.denied_permissions),
                    },
                    position: r.position as u16,
                    flags: RoleFlags::from_bits_truncate(r.flags as u32),
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    let members = if members {
        let members: Vec<MemberQueryResponse> = sqlx::query_as!(
            MemberQueryResponse,
            r#"SELECT
                m.id AS "id: PostgresU128",
                m.nick AS nick,
                m.joined_at AS joined_at,
                u.username AS username,
                u.discriminator AS discriminator,
                u.avatar AS avatar,
                u.banner AS banner,
                u.bio AS bio,
                u.flags AS flags
            FROM
                members m
            CROSS JOIN LATERAL (
                SELECT * FROM users u WHERE u.id = m.id
            ) AS u
            WHERE
                guild_id = $1
            "#,
            PostgresU128::new(guild_id) as _,
        )
        .fetch_all(db)
        .await
        .promote(&headers)?;

        let member_roles: Vec<MemberRoleQueryResponse> = sqlx::query_as!(
            MemberRoleQueryResponse,
            r#"SELECT
                role_id AS "role_id: PostgresU128",
                user_id AS "user_id: PostgresU128"
            FROM
                role_data
            WHERE
                guild_id = $1
            "#,
            PostgresU128::new(guild_id) as _,
        )
        .fetch_all(db)
        .await
        .promote(&headers)?;

        let member_roles = member_roles
            .into_iter()
            .into_group_map_by(|r| r.user_id.to_u128());

        Some(
            members
                .into_iter()
                .map(|m| Member {
                    user: MaybePartialUser::Full(User {
                        id: m.id.to_u128(),
                        username: m.username,
                        discriminator: m.discriminator as u16,
                        avatar: m.avatar,
                        banner: m.banner,
                        bio: m.bio,
                        flags: UserFlags::from_bits_truncate(m.flags as u32),
                    }),
                    guild_id,
                    nick: m.nick,
                    roles: member_roles
                        .get(&m.id.to_u128())
                        .map(|r| r.iter().map(|r| r.role_id.to_u128()).collect::<Vec<_>>()),
                    joined_at: m.joined_at,
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };

    Response::ok(Guild {
        partial,
        members,
        roles,
        channels,
        vanity_url: guild.vanity_url,
    })
    .promote_ok(&headers)
}

/// DELETE /guilds/:id
#[allow(clippy::cast_sign_loss)]
pub async fn delete_guild(
    Auth(user_id, _): Auth,
    Path(guild_id): Path<u128>,
    json: Option<Json<DeleteGuildPayload>>,
    headers: HeaderMap,
) -> HeaderAwareResult<StatusCode> {
    struct OwnerIdQueryResponse {
        owner_id: PostgresU128,
    }

    let db = get_pool();
    let owner_id: OwnerIdQueryResponse = sqlx::query_as!(
        OwnerIdQueryResponse,
        r#"SELECT
            owner_id AS "owner_id: PostgresU128"
        FROM
            guilds
        WHERE
            id = $1
        "#,
        PostgresU128::new(guild_id) as _,
    )
    .fetch_optional(db)
    .await
    .promote(&headers)?
    .ok_or_else(|| Response::not_found("guild", format!("Guild with ID {} not found", guild_id)))
    .promote(&headers)?;

    if owner_id.owner_id.to_u128() != user_id {
        return Response(
            StatusCode::FORBIDDEN,
            Error::NotOwner {
                guild_id,
                message: "You must be the owner of the guild to perform this action",
            },
        )
        .promote_err(&headers);
    }

    let user = sqlx::query!(
        "SELECT flags, password FROM users WHERE id = $1",
        PostgresU128::new(user_id) as _,
    )
    .fetch_one(db)
    .await
    .promote(&headers)?;

    if !UserFlags::from_bits_truncate(user.flags as u32).contains(UserFlags::BOT) {
        let Json(DeleteGuildPayload { password }) = json
            .ok_or(Response(
                StatusCode::BAD_REQUEST,
                Error::MissingBody {
                    message: "Missing request body, which is required for user accounts",
                },
            ))
            .promote(&headers)?;

        if !argon2_async::verify(password, user.password.unwrap_or_default())
            .await
            .promote(&headers)?
        {
            return Response(
                StatusCode::UNAUTHORIZED,
                Error::InvalidCredentials {
                    what: "password",
                    message: "Invalid password",
                },
            )
            .promote_err(&headers);
        }
    }

    sqlx::query!(
        "DELETE FROM guilds WHERE id = $1",
        PostgresU128::new(guild_id) as _,
    )
    .execute(db)
    .await
    .promote(&headers)?;

    Ok(StatusCode::NO_CONTENT)
}

#[must_use]
pub fn router() -> Router {
    Router::new()
        .route("/guilds", post(create_guild.layer(ratelimit!(2, 30))))
        .route(
            "/guilds/:id",
            get(get_guild.layer(ratelimit!(2, 15))).delete(delete_guild.layer(ratelimit!(3, 18))),
        )
}
