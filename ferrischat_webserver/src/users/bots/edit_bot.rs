use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::BotUpdateJson;
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, User, UserFlags};
use sqlx::types::BigDecimal;

/// PATCH `/api/v0/users/{user_id}/bots/{bot_id}`
/// Edits the bot with the attached payload
pub async fn edit_bot(
    req: HttpRequest,
    bot_info: Json<BotUpdateJson>,
    auth: crate::Authorization,
) -> impl Responder {
    let bot_id = get_item_id!(req, "bot_id");
    let bigint_bot_id = u128_to_bigdecimal!(bot_id);

    let BotUpdateJson { username } = bot_info.0;

    let db = get_db_or_fail!();

    let bigdecimal_owner_id: BigDecimal = match sqlx::query!(
        "SELECT owner_id FROM bots WHERE user_id = $1",
        bigint_bot_id,
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(r)) => r.owner_id,
        Ok(None) => {
            return HttpResponse::NotFound().json(NotFoundJson {
                message: format!("Unknown bot with id {}", bot_id),
            })
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned a error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };

    let owner_id = bigdecimal_to_u128!(bigdecimal_owner_id);

    if owner_id != auth.0 {
        return HttpResponse::Forbidden().finish();
    }

    if let Some(username) = username {
        if let Err(e) = sqlx::query!(
            "UPDATE users SET name = $1 WHERE id = $2",
            username,
            bigint_bot_id
        )
        .execute(db)
        .await
        {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            });
        }
    }

    let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_bot_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(Some(user)) => HttpResponse::Ok().json(User {
            id: bot_id,
            name: user.name.clone(),
            avatar: None,
            guilds: None,
            flags: UserFlags::from_bits_truncate(user.flags),
            discriminator: user.discriminator,
        }),
        Ok(None) => HttpResponse::NotFound().json(NotFoundJson {
            message: format!("Unknown bot with id {}", bot_id),
        }),
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    }
}
