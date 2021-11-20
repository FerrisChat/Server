use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::BotUpdateJson;
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, User, UserFlags};

use tokio::sync::oneshot::channel;

/// PATCH /api/v0/users/{user_id}/bots/{bot_id}
/// Edits the bot with the attached payload
pub async fn edit_bot(
    req: HttpRequest,
    bot_info: Json<BotUpdateJson>,
    auth: crate::Authorization,
) -> impl Responder {
    let user_id = get_item_id!(req, "bot_id");
    let bigdecimal_user_id = u128_to_bigdecimal!(user_id);

    let bigint_user_id = u128_to_bigdecimal!(user_id);

    let BotUpdateJson {
        username,
    } = bot_info.0;

    let db = get_db_or_fail!();

    let owner_id_resp = match sqlx::query!(
        "SELECT * FROM bots WHERE user_id = $1",
        bigdecimal_user_id,
    )
        .fetch_one(db)
        .await
    {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e)
        })
    };

    let u128_owner_id = bigdecimal_to_u128!(owner_id_resp.owner_id);

    if u128_owner_id != auth.0 {
        return HttpResponse::Forbidden().finish()
    }

    if let Some(username) = username {
        let resp = sqlx::query!(
            "UPDATE users SET name = $1 WHERE id = $2",
            username,
            bigint_user_id
        )
            .execute(db)
            .await;
        match resp {
            Ok(_) => (),
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned an error: {}", e),
                })
            }
        }
    }

    let resp = sqlx::query!("SELECT * FROM users WHERE id = $1", bigint_user_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(user) => HttpResponse::Ok().json(User {
                id: user_id,
                name: user.name.clone(),
                avatar: None,
                guilds: None,
                flags: UserFlags::from_bits_truncate(user.flags),
                discriminator: user.discriminator,
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "User not found".to_string(),
            }),
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned an error: {}", e),
            })
        }
    }
}