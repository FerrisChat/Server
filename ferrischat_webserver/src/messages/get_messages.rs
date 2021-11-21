use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, Message, NotFoundJson, User, UserFlags};

/// GET /api/v0/guilds/{guild_id}/channels/{channel_id}/messages/{message_id}
pub async fn get_message(req: HttpRequest, _: crate::Authorization) -> impl Responder {
    let message_id = get_item_id!(req, "message_id");
    let bigint_message_id = u128_to_bigdecimal!(message_id);

    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);

    let db = get_db_or_fail!();

    let resp = sqlx::query!(
        "SELECT m.*, a.name AS author_name, a.flags AS author_flags, a.discriminator AS author_discriminator FROM messages m CROSS JOIN LATERAL (SELECT * FROM users WHERE id = m.author_id) AS a WHERE m.id = $1 AND m.channel_id = $2",
        bigint_message_id,
        bigint_channel_id,
    )
    .fetch_optional(db)
    .await;

    match resp {
        Ok(r) => match r {
            Some(m) => {
                let author_id = bigdecimal_to_u128!(m.author_id);

                HttpResponse::Ok().json(Message {
                    id: message_id,
                    content: m.content,
                    channel_id,
                    author_id,
                    edited_at: m.edited_at,
                    embeds: vec![],
                    author: Some(User {
                        id: author_id,
                        name: m.author_name,
                        avatar: None,
                        guilds: None,
                        flags: UserFlags::from_bits_truncate(m.author_flags),
                        discriminator: m.author_discriminator,
                    }),
                    nonce: None,
                })
            }
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "message not found".to_string(),
            }),
        },
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        }),
    }
}
