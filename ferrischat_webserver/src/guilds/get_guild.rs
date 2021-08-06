use crate::get_db_or_fail;
use actix_web::{
    web::{Json, Path},
    HttpResponse, Responder,
};
use ferrischat_common::types::{Guild, InternalServerErrorJson};
use num_traits::cast::ToPrimitive;
use sqlx::types::BigDecimal;

/// GET /api/v1/guilds/{id}
pub async fn get_guild(Path(guild_id): Path<i64>) -> impl Responder {
    let db = get_db_or_fail!();
    let guild_id = BigDecimal::from(guild_id);
    let resp = sqlx::query!("SELECT * FROM guilds WHERE id = $1", guild_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(guild) => HttpResponse::Ok().json(Guild {
                id: match guild
                    .id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128()
                {
                    Some(id) => id,
                    None => {
                        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                            reason: "snowflake ID overflowed 128 bit integer".to_string(),
                        })
                    }
                },
                owner_id: guild.owner_id,
                name: guild.name,
                channels: None,
                members: None,
            }),
            None => HttpResponse::NotFound().finish(),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
