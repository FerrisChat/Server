use crate::ws::{fire_event, WsEventError};

use ferrischat_common::ws::WsOutboundEvent;

use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::ChannelCreateJson;
use ferrischat_common::types::{Channel, InternalServerErrorJson, ModelType};
use ferrischat_macros::get_db_or_fail;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST `/api/v0/guilds/{guild_id/channels`
pub async fn create_channel(
    _: crate::Authorization,
    channel_info: Json<ChannelCreateJson>,
    req: HttpRequest,
) -> impl Responder {
    let db = get_db_or_fail!();

    let ChannelCreateJson { name } = channel_info.0;

    let node_id = get_node_id!();
    let channel_id = generate_snowflake::<0>(ModelType::Channel as u8, node_id);
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let resp = sqlx::query!(
        "INSERT INTO channels VALUES ($1, $2, $3)",
        bigint_channel_id,
        name,
        bigint_guild_id
    )
    .execute(db)
    .await;

    let channel_obj = match resp {
        Ok(_) => Channel {
            id: channel_id,
            name,
            guild_id,
        },
        Err(e) => {
            return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned a error: {}", e),
                is_bug: false,
                link: None,
            })
        }
    };

    let event = WsOutboundEvent::ChannelCreate {
        channel: channel_obj.clone(),
    };

    if let Err(e) = fire_event(format!("channel_{}_{}", guild_id, channel_id), &event).await {
        let reason = match e {
            WsEventError::MissingRedis => "Redis pool missing".to_string(),
            WsEventError::RedisError(e) => format!("Redis returned an error: {}", e),
            WsEventError::JsonError(e) => {
                format!("Failed to serialize message to JSON format: {}", e)
            }
            WsEventError::PoolError(e) => format!("`deadpool` returned an error: {}", e),
        };
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason,
            is_bug: true,
            link: Some(
                "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+failed+to+fire+event"
                    .to_string(),
            ),
        });
    }

    HttpResponse::Created().json(channel_obj)
}
