use crate::{get_pool, Error, PostgresU128, Response, StatusCode};
use common::models::PermissionOverwrite;
use common::{
    models::{PermissionPair, Permissions, Role},
    util::calculate_permissions,
};

/// Asserts that the guild with the provided ID exists.
pub async fn assert_guild_exists(guild_id: u128) -> Result<(), Response<Error>> {
    let pool = get_pool();

    let exists = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM guilds WHERE id = $1) AS exists",
        PostgresU128::new(guild_id) as _,
    )
    .fetch_one(pool)
    .await?
    .exists
    .unwrap_or(false);

    if !exists {
        return Err(Response(
            StatusCode::NOT_FOUND,
            Error::<u128>::NotFound {
                entity: "guild",
                message: format!("Guild with ID {} not found", guild_id),
            },
        ));
    }

    Ok(())
}

/// Asserts that the user is a member of the guild.
pub async fn assert_member(guild_id: u128, user_id: u128) -> Result<(), Response<Error>> {
    assert_guild_exists(guild_id).await?;
    let db = get_pool();

    if !sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM members WHERE guild_id = $1 AND id = $2) AS exists",
        PostgresU128::new(guild_id) as _,
        PostgresU128::new(user_id) as _,
    )
    .fetch_one(db)
    .await?
    .exists
    .unwrap_or(false)
    {
        return Err(Response(
            StatusCode::FORBIDDEN,
            Error::NotMember {
                guild_id,
                message: "You must be a member of this guild to perform this action",
            },
        ));
    }

    Ok(())
}

/// Calculates the permissions of a member in a guild.
#[allow(clippy::cast_sign_loss, clippy::default_trait_access)]
pub async fn get_permissions(
    guild_id: u128,
    user_id: u128,
    channel_id: Option<u128>,
) -> Result<Permissions, Response<Error>> {
    struct QueryResult {
        id: PostgresU128,
        allowed_permissions: i64,
        denied_permissions: i64,
        position: i16,
    }

    struct OverwriteQueryResult {
        target_id: PostgresU128,
        allow: i64,
        deny: i64,
    }

    assert_guild_exists(guild_id).await?;
    let db = get_pool();

    let roles: Vec<QueryResult> = sqlx::query_as!(
        QueryResult,
        r#"
        SELECT
            id AS "id: PostgresU128",
            allowed_permissions,
            denied_permissions,
            position
        FROM roles
        WHERE id
        IN (
            SELECT role_id
            FROM role_data
            WHERE user_id = $1 AND guild_id = $2
        )"#,
        PostgresU128::new(user_id) as _,
        PostgresU128::new(guild_id) as _,
    )
    .fetch_all(db)
    .await?;

    let mut roles = roles
        .into_iter()
        .map(|role| Role {
            id: role.id.to_u128(),
            permissions: PermissionPair {
                allow: Permissions::from_bits_truncate(role.allowed_permissions),
                deny: Permissions::from_bits_truncate(role.denied_permissions),
            },
            position: role.position as u16,
            name: String::new(),
            color: None,
            guild_id: 0,
            flags: Default::default(),
        })
        .collect::<Vec<_>>();

    let overwrites = match channel_id {
        Some(channel_id) => {
            let overwrites: Vec<OverwriteQueryResult> = sqlx::query_as!(
                OverwriteQueryResult,
                r#"
                SELECT
                    target_id AS "target_id: PostgresU128",
                    allow,
                    deny
                FROM channel_overwrites
                WHERE channel_id = $1"#,
                PostgresU128::new(channel_id) as _,
            )
            .fetch_all(db)
            .await?;

            Some(
                overwrites
                    .into_iter()
                    .map(|o| PermissionOverwrite {
                        id: o.target_id.to_u128(),
                        permissions: PermissionPair {
                            allow: Permissions::from_bits_truncate(o.allow),
                            deny: Permissions::from_bits_truncate(o.deny),
                        },
                    })
                    .collect::<Vec<_>>(),
            )
        }
        None => None,
    };

    Ok(calculate_permissions(
        user_id,
        &mut roles,
        overwrites.as_ref().map(AsRef::as_ref),
    ))
}

/// Asserts that the user has the given permissions. If a channel ID is provided, then the
/// permission overwrites for that channel are also applied, along with its parents.
pub async fn assert_permissions(
    guild_id: u128,
    user_id: u128,
    channel_id: Option<u128>,
    permissions: Permissions,
) -> Result<(), Response<Error>> {
    if !permissions.contains(get_permissions(guild_id, user_id, channel_id).await?) {
        return Err(Response(
            StatusCode::FORBIDDEN,
            Error::<u128>::MissingPermissions {
                message: "You are missing permissions to perform this action",
            },
        ));
    }

    Ok(())
}

/// Asserts that the user is the owner of the guild.
pub async fn assert_guild_owner(guild_id: u128, user_id: u128) -> Result<(), Response<Error>> {
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
    .await?
    .ok_or_else(|| Response::not_found("guild", format!("Guild with ID {} not found", guild_id)))?;

    if owner_id.owner_id.to_u128() != user_id {
        return Err(Response(
            StatusCode::FORBIDDEN,
            Error::<u128>::MissingPermissions {
                message: "You are missing permissions to perform this action",
            },
        ));
    }

    Ok(())
}
