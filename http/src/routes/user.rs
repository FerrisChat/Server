use crate::{get_pool, ratelimit, Error, PostgresU128, PromoteErr, Response, RouteResult};
use common::{
    http::{CreateUserPayload, CreateUserResponse},
    models::ModelType,
};

use axum::{
    extract::Json, handler::Handler, headers::HeaderMap, http::StatusCode, routing::post, Router,
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

#[must_use]
pub fn router() -> Router {
    Router::new().route("/users", post(create_user.layer(ratelimit!(3, 15))))
}
