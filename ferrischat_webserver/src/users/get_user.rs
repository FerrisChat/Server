use actix_web::{web::Path, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, User};
use ferrischat_macros::{bigdecimal_to_u128, get_db_or_fail};
use num_traits::cast::ToPrimitive;
use sqlx::types::BigDecimal;

/// GET /api/v0/users/{user_id}
pub async fn get_user(Path(user_id): Path<i64>, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let user_id = BigDecimal::from(user_id);
    let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(user) => HttpResponse::Ok().json(User {
                id: bigdecimal_to_u128!(user.id),
                name: user.name,
                guilds: None,
                flags: 0,
            }),
            None => HttpResponse::NotFound().finish(),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
