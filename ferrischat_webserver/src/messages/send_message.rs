use actix_web::web::Json;
use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::request_json::MessageCreateJson;
use ferrischat_common::types::{BadRequestJson, InternalServerErrorJson, Message, ModelType};
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/{guild_id}/channels/{channel_id}/messages
pub async fn create_message(
    auth: crate::Authorization,
    req: HttpRequest,
    json: Json<MessageCreateJson>,
) -> impl Responder {
    let content = json.content.clone();
    if content.len() > 10240 {
        return HttpResponse::BadRequest().json(BadRequestJson {
            reason: "message content size must be fewer than 10,240 bytes".to_string(),
            location: None,
        });
    }

    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let message_id = generate_snowflake::<0>(ModelType::Message as u8, 0);
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let author_id = auth.0;
    let bigint_author_id = u128_to_bigdecimal!(author_id);

    let db = get_db_or_fail!();
    let resp = sqlx::query!(
        "INSERT INTO messages VALUES ($1, $2, $3, $4)",
        bigint_message_id,
        content,
        bigint_channel_id,
        bigint_author_id
    )
    .execute(db)
    .await;
    if let Err(e) = resp {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        });
    }

    HttpResponse::Created().json(Message {
        id: message_id,
        content,
        channel_id,
        author_id,
    })
}
