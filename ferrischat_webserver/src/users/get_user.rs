use actix_web::{web::Path, HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, User};
use ferrischat_macros::{bigdecimal_to_u128, get_db_or_fail};
use num_traits::cast::ToPrimitive;
use sqlx::types::BigDecimal;

/// GET /api/v0/users/{user_id}
pub async fn get_user(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let user_id = get_item_id!(req, "user_id");
    let db = get_db_or_fail!();
    let bigint_user_id = BigDecimal::from(user_id);
    let authorized_user = auth.0;
    let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(user) => HttpResponse::Ok().json(User {
                id: user_id,
                name: user.name,
                guilds: if authorized_user == user_id {
                    user.guilds
                } else {
                    None
                },
                flags: user.flags,
            }),
            None => HttpResponse::NotFound().finish(),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
