use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::ChannelUpdateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, NotFoundJson};

pub async fn edit_channel(
    req: HttpRequest,
    channel_info: Json<ChannelUpdateJson>,
    _: crate::Authorization,
) -> impl Responder {
    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let db = get_db_or_fail!();

    let ChannelUpdateJson { name } = channel_info.0;

    let resp = sqlx::query!(
        "UPDATE channels SET name = $1 WHERE id = $2 RETURNING *",
        name,
        bigint_channel_id
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(resp) => match resp {
            Some(channel) => HttpResponse::Ok().json(Channel {
                id: bigdecimal_to_u128!(channel.id),
                name: channel.name.clone(),
                guild_id: bigdecimal_to_u128!(channel.guild_id),
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Channel not found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned an error: {}", e),
        }),
    }
}