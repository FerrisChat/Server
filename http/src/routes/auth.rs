use crate::{get_pool, ratelimit, Error, PostgresU128, PromoteErr, Response, RouteResult};
use common::{
    http::{LoginRequest, LoginResponse, TokenRetrievalMethod},
    models::UserFlags,
};

use axum::{
    extract::Json, handler::Handler, headers::HeaderMap, http::StatusCode, routing::post, Router,
};

/// POST /login
pub async fn login(
    Json(LoginRequest {
        email,
        password,
        method,
    }): Json<LoginRequest>,
    headers: HeaderMap,
) -> RouteResult<LoginResponse> {
    struct LoginSqlQuery {
        id: PostgresU128,
        password: Option<String>,
        flags: i32,
    }

    let db = get_pool();

    let user: LoginSqlQuery = sqlx::query_as!(
        LoginSqlQuery,
        r#"SELECT id as "id: PostgresU128", password, flags FROM users WHERE email = $1"#,
        email,
    )
    .fetch_optional(db)
    .await
    .promote(&headers)?
    .ok_or_else(|| Response::not_found("user", "User with the given email not found".into()))
    .promote(&headers)?;

    #[allow(clippy::cast_sign_loss)]
    let flags = UserFlags::from_bits_truncate(user.flags as u32);
    if flags.contains(UserFlags::BOT) {
        return Response(
            StatusCode::FORBIDDEN,
            Error::UnsupportedAuthMethod {
                message: "Bots cannot login with this method, use a bot token instead",
            },
        )
        .promote_err(&headers);
    }

    if !argon2_async::verify(password, user.password.unwrap_or_default())
        .await
        .promote(&headers)?
    {
        return Response(
            StatusCode::UNAUTHORIZED,
            Error::InvalidCredentials {
                what: "password",
                message: "Password is incorrect",
            },
        )
        .promote_err(&headers);
    }

    if method == TokenRetrievalMethod::Reuse {
        if let Some(response) = sqlx::query!(
            r#"SELECT token FROM tokens WHERE user_id = $1"#,
            user.id as _,
        )
        .fetch_optional(db)
        .await
        .promote(&headers)?
        {
            return Response::ok(LoginResponse {
                token: response.token,
                user_id: user.id.to_u128(),
            })
            .promote_ok(&headers);
        }
    }

    let mut transaction = db.begin().await.promote(&headers)?;

    if method == TokenRetrievalMethod::Revoke {
        sqlx::query!(r#"DELETE FROM tokens WHERE user_id = $1"#, user.id as _,)
            .execute(&mut transaction)
            .await
            .promote(&headers)?;
    }

    let token = crate::auth::generate_token(user.id.to_u128());

    sqlx::query!(
        r#"INSERT INTO tokens (user_id, token) VALUES ($1, $2)"#,
        user.id as _,
        token,
    )
    .execute(&mut transaction)
    .await
    .promote(&headers)?;

    transaction.commit().await.promote(&headers)?;

    Response::ok(LoginResponse {
        token,
        user_id: user.id.to_u128(),
    })
    .promote_ok(&headers)
}

#[must_use]
pub fn router() -> Router {
    Router::new().route("/login", post(login.layer(ratelimit!(3, 10))))
}
