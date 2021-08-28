use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::ChannelUpdateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, NotFoundJson};

use num_traits::ToPrimitive;

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
        "UPDATE channels SET name = $2 WHERE id = $1 RETURNING *",
        bigint_channel_id,
        name
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(resp) => match resp {
            Some(channel) => HttpResponse::Ok().json(Channel {
                id: channel
                    .id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128(),
                name: channel.name.clone(),
                guild_id: channel
                    .guild_id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128(),
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
