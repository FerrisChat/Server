use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson};

/// DELETE /api/v0/users/{user_id}/bots/{bot_id}
/// Deletes the bot
pub async fn delete_bot(req: HttpRequest, auth: crate::Authorization) -> impl Responder {
    let user_id = get_item_id!(req, "bot_id");
    let bigdecimal_user_id = u128_to_bigdecimal!(user_id);

    let bigint_user_id = u128_to_bigdecimal!(user_id);

    let db = get_db_or_fail!();

    let owner_id_resp =
        match sqlx::query!("SELECT * FROM bots WHERE user_id = $1", bigdecimal_user_id,)
            .fetch_one(db)
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: format!("DB returned a error: {}", e),
                    is_bug: false,
                    link: None,
                })
            }
        };

    let u128_owner_id = bigdecimal_to_u128!(owner_id_resp.owner_id);

    if u128_owner_id != auth.0 {
        return HttpResponse::Forbidden().finish();
    }

    // Drop the user.
    let resp = sqlx::query!(
        "DELETE FROM users WHERE id = $1 RETURNING (id)",
        bigint_user_id
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(r) => match r {
            Some(_) => HttpResponse::NoContent().finish(),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: format!("Unknown user with id {}", user_id),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB Returned a error: {}", e),
            is_bug: false,
            link: None,
        }),
    }
}
