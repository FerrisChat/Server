use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{Channel, InternalServerErrorJson, NotFoundJson};

/// GET /api/v0/guilds/{guild_id/channels/{channel_id}
pub async fn get_channel(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let db = get_db_or_fail!();
    let channel_id = u128_to_bigdecimal!(get_item_id!(req, "channel_id"));
    let resp = sqlx::query!("SELECT * FROM channels WHERE id = $1", channel_id)
        .fetch_optional(db)
        .await;

    match resp {
        Ok(resp) => match resp {
            Some(channel) => HttpResponse::Ok().json(Channel {
                id: bigdecimal_to_u128!(channel.id),
                name: channel.name,
                guild_id: bigdecimal_to_u128!(channel.guild_id),
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Channel Not Found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("database returned a error: {}", e),
        }),
    }
}
