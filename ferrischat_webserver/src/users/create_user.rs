use actix_web::{HttpResponse, Responder};
use sqlx::types::BigDecimal;
use ferrischat_macros::{ get_db_or_fail};
use ferrischat_snowflake_generator::generate_snowflake;
use ferrischat_common::types::{User, InternalServerErrorJson};
use crate::API_VERSION;
use num_traits::FromPrimitive;

/// POST /api/v1/users/
pub async fn create_user() -> impl Responder {
    let db = get_db_or_fail!();
    let user_id = generate_snowflake::<0>(0, 0);
    match sqlx::query!("INSERT INTO users VALUES ($1, $2, null, $3)", BigDecimal::from_u128(user_id), "New User", 0).execute(db).await {
        Ok(r) => {HttpResponse::Created().json(User {
            id: user_id,
            name: "New User".to_string(),
            guilds: None,
            flags: 0,
        })}
        Err(e) => {HttpResponse::InternalServerError().json(InternalServerErrorJson { reason: format!("DB returned a error: {}", e) })}
    }
}