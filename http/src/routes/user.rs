use crate::{
    get_pool, ratelimit, Auth, Error, HeaderAwareResult, PostgresU128, PromoteErr, Response,
    RouteResult,
};
use common::{
    http::{CreateUserPayload, CreateUserResponse, DeleteUserPayload, EditUserPayload},
    models::{ClientUser, ModelType, User, UserFlags},
};

use axum::{
    extract::{Json, Path},
    handler::Handler,
    headers::HeaderMap,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use ferrischat_snowflake_generator::generate_snowflake;

fn validate_username<T: AsRef<str>>(username: T) -> Result<(), Error> {
    let username = username.as_ref();
    let length = username.chars().count();

    if length < 2 {
        return Err(Error::InvalidUsername {
            message: "Username must be at least 2 characters long".to_string(),
        });
    }

    if length > 32 {
        return Err(Error::InvalidUsername {
            message: "Username cannot be longer than 32 characters".to_string(),
        });
    }

    for forbidden in ['\n', '\r', '#', '@'] {
        if username.contains(forbidden) {
            return Err(Error::InvalidUsername {
                message: format!("Username cannot contain {:?}", forbidden),
            });
        }
    }

    Ok(())
}

/// POST /users
pub async fn create_user(
    headers: HeaderMap,
    Json(CreateUserPayload {
        username,
        email,
        password,
    }): Json<CreateUserPayload>,
) -> RouteResult<CreateUserResponse> {
    validate_username(&username)
        .map_err(|err| Response(StatusCode::BAD_REQUEST, err).promote(&headers))?;

    let db = get_pool();
    if sqlx::query!("SELECT email FROM users WHERE email = $1", email,)
        .fetch_optional(db)
        .await
        .promote(&headers)?
        .is_some()
    {
        return Response(
            StatusCode::CONFLICT,
            Error::AlreadyTaken {
                what: "email",
                message: "Email is already in use".to_string(),
            },
        )
        .promote_err(&headers);
    }

    let mut transaction = db.begin().await.promote(&headers)?;

    // TODO: node id
    let id = generate_snowflake::<0>(ModelType::User as u8, 0);
    let hashed = argon2_async::hash(password).await.promote(&headers)?;

    let discriminator = sqlx::query!(
        "INSERT INTO
            users (id, username, email, password)
        VALUES
            ($1, $2, $3, $4)
        RETURNING
            discriminator",
        PostgresU128::new(id) as _,
        username,
        email,
        hashed,
    )
    .fetch_optional(&mut transaction)
    .await
    .promote(&headers)?;

    if discriminator.is_none() {
        transaction.rollback().await.promote(&headers)?;
        return Response(
            StatusCode::CONFLICT,
            Error::AlreadyTaken {
                what: "username",
                message: "Username is already taken".to_string(),
            },
        )
        .promote_err(&headers);
    }

    let token = crate::auth::generate_token(id);

    sqlx::query!(
        r#"INSERT INTO tokens (user_id, token) VALUES ($1, $2)"#,
        PostgresU128::new(id) as _,
        token,
    )
    .execute(&mut transaction)
    .await
    .promote(&headers)?;

    transaction.commit().await.promote(&headers)?;

    Response::created(CreateUserResponse { id, token }).promote_ok(&headers)
}

/// GET /users/me
#[allow(clippy::cast_sign_loss)]
pub async fn get_client_user(Auth(id, _): Auth, headers: HeaderMap) -> RouteResult<ClientUser> {
    let db = get_pool();

    let data = sqlx::query!(
        "SELECT
            username,
            discriminator,
            email,
            avatar,
            banner,
            bio,
            flags
        FROM
            users
        WHERE
            id = $1",
        PostgresU128::new(id) as _,
    )
    .fetch_one(db)
    .await
    .promote(&headers)?;

    let user = User {
        id,
        username: data.username,
        discriminator: data.discriminator as u16,
        avatar: data.avatar,
        banner: data.banner,
        bio: data.bio,
        flags: UserFlags::from_bits_truncate(data.flags as u32),
    };

    Response::ok(ClientUser {
        user,
        email: data.email,
        guilds: Vec::new(),        // TODO
        relationships: Vec::new(), // TODO
        folders: None,             // TODO
    })
    .promote_ok(&headers)
}

/// PATCH /users/me
#[allow(clippy::cast_sign_loss)]
pub async fn edit_user(
    Auth(id, _): Auth,
    Json(payload): Json<EditUserPayload>,
    headers: HeaderMap,
) -> RouteResult<User> {
    let db = get_pool();
    let user = sqlx::query!(
        "SELECT
            username,
            discriminator,
            avatar,
            banner,
            bio,
            flags
        FROM
            users
        WHERE
            id = $1",
        PostgresU128::new(id) as _,
    )
    .fetch_one(db)
    .await
    .promote(&headers)?;

    let mut transaction = db.begin().await.promote(&headers)?;

    macro_rules! update {
        ($query:literal, $field:ident, $value:expr) => {{
            sqlx::query!($query, $value, PostgresU128::new(id) as _)
                .execute(&mut transaction)
                .await
                .promote(&headers)?;

            $field
        }};
    }

    // TODO: the username might overlap with a discriminator
    let username = if let Some(username) = payload.username {
        update!(
            "UPDATE users SET username = $1 WHERE id = $2",
            username,
            username
        )
    } else {
        user.username
    };

    let avatar = if payload.avatar.is_absent() {
        user.avatar
    } else {
        let avatar = Option::from(payload.avatar);
        update!("UPDATE users SET avatar = $1 WHERE id = $2", avatar, avatar)
    };

    let banner = if payload.banner.is_absent() {
        user.banner
    } else {
        let banner = Option::from(payload.banner);
        update!("UPDATE users SET banner = $1 WHERE id = $2", banner, banner)
    };

    let bio = if payload.bio.is_absent() {
        user.bio
    } else {
        let bio = Option::from(payload.bio);
        update!("UPDATE users SET bio = $1 WHERE id = $2", bio, bio)
    };

    transaction.commit().await.promote(&headers)?;

    Response::ok(User {
        id,
        username,
        discriminator: user.discriminator as u16,
        avatar,
        banner,
        bio,
        flags: UserFlags::from_bits_truncate(user.flags as u32),
    })
    .promote_ok(&headers)
}

/// DELETE /users/me
pub async fn delete_user(
    Auth(id, _): Auth,
    Json(DeleteUserPayload { password }): Json<DeleteUserPayload>,
    headers: HeaderMap,
) -> HeaderAwareResult<StatusCode> {
    let db = get_pool();

    let hashed: String = sqlx::query!(
        "SELECT password FROM users WHERE id = $1",
        PostgresU128::new(id) as _,
    )
    .fetch_one(db)
    .await
    .promote(&headers)?
    .password
    .ok_or_else(|| {
        Response(
            StatusCode::FORBIDDEN,
            Error::UnsupportedAuthMethod {
                message: "This user is a bot account, but this endpoint can only delete user \
                    accounts. To delete bot accounts, see the DELETE /bots/:id endpoint.",
            },
        )
        .promote(&headers)
    })?;

    if !argon2_async::verify(hashed, password)
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

    sqlx::query!(
        "DELETE FROM users WHERE id = $1",
        PostgresU128::new(id) as _
    )
    .execute(db)
    .await
    .promote(&headers)?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /users/:id
#[allow(clippy::cast_sign_loss)]
pub async fn get_user(_: Auth, headers: HeaderMap, Path(id): Path<u128>) -> RouteResult<User> {
    let db = get_pool();

    let data = sqlx::query!(
        "SELECT
            username,
            discriminator,
            avatar,
            banner,
            bio,
            flags
        FROM
            users
        WHERE
            id = $1",
        PostgresU128::new(id) as _,
    )
    .fetch_one(db)
    .await
    .promote(&headers)?;

    Response::ok(User {
        id,
        username: data.username,
        discriminator: data.discriminator as u16,
        avatar: data.avatar,
        banner: data.banner,
        bio: data.bio,
        flags: UserFlags::from_bits_truncate(data.flags as u32),
    })
    .promote_ok(&headers)
}

#[must_use]
pub fn router() -> Router {
    Router::new()
        .route("/users", post(create_user.layer(ratelimit!(3, 15))))
        .route(
            "/users/me",
            get(get_client_user.layer(ratelimit!(3, 5)))
                .patch(edit_user.layer(ratelimit!(2, 15)))
                .delete(delete_user.layer(ratelimit!(2, 40))),
        )
        .route("/users/:id", get(get_user.layer(ratelimit!(3, 5))))
}
