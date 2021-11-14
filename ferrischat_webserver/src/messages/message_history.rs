use actix_web::{web::Query, HttpRequest, HttpResponse, Responder};

use ferrischat_common::request_json::GetMessageHistoryParams;
use ferrischat_common::types::{BadRequestJson, InternalServerErrorJson, Message, MessageHistory};

use num_traits::ToPrimitive;

/// GET /api/v0/channels/{channel_id}/messages
pub async fn get_message_history(
    req: HttpRequest,
    _: crate::Authorization,
    params: Query<GetMessageHistoryParams>,
) -> impl Responder {
    let channel_id = get_item_id!(req, "channel_id");
    let bigint_channel_id = u128_to_bigdecimal!(channel_id);
    let db = get_db_or_fail!();

    let GetMessageHistoryParams {
        mut limit,
        oldest_first,
    } = params.0;

    if limit < Some(0) {
        return HttpResponse::BadRequest().json(BadRequestJson {
            reason: "limit must be > 0".to_string(),
            location: None,
        });
    }

    if limit >= Some(9223372036854775807) {
        limit = None;
    }

    let messages = {
        if oldest_first == Some(true) {
            let resp = sqlx::query!(
                "SELECT m.*, (SELECT u.* FROM users u WHERE id = m.author_id) AS author FROM messages m WHERE channel_id = $1 ORDER BY id ASC LIMIT $2",
                bigint_channel_id,
                limit
            )
            .fetch_all(db)
            .await;

            match resp {
                Ok(mut resp) => resp
                    .iter_mut()
                    .filter_map(|x| {
                        let content = std::mem::take(&mut x.content);
                        Some(Message {
                            id: x.id.with_scale(0).into_bigint_and_exponent().0.to_u128()?,
                            content,
                            channel_id: x
                                .channel_id
                                .with_scale(0)
                                .into_bigint_and_exponent()
                                .0
                                .to_u128()?,
                            author_id: x
                                .author_id
                                .with_scale(0)
                                .into_bigint_and_exponent()
                                .0
                                .to_u128()?,
                            author: User {
                                id: x.author.id,
                                name: x.author.name,
                                avatar: None,
                                guilds: None,
                                flags: UserFlags::from_bits_truncate(x.author.flags),
                                discriminator: x.author.discriminator,
                            },
                            edited_at: x.edited_at,
                            embeds: vec![],
                            nonce: None,
                        })
                    })
                    .collect(),
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("database returned a error: {}", e),
                    })
                }
            }
        } else {
            let resp = sqlx::query!(
                "SELECT m.*, (SELECT u.* FROM users u WHERE id = m.author_id) AS author FROM messages m WHERE channel_id = $1 ORDER BY id DESC LIMIT $2",
                bigint_channel_id,
                limit
            )
            .fetch_all(db)
            .await;

            match resp {
                Ok(mut resp) => resp
                    .iter_mut()
                    .filter_map(|x| {
                        let content = std::mem::take(&mut x.content);
                        Some(Message {
                            id: x.id.with_scale(0).into_bigint_and_exponent().0.to_u128()?,
                            content,
                            channel_id: x
                                .channel_id
                                .with_scale(0)
                                .into_bigint_and_exponent()
                                .0
                                .to_u128()?,
                            author_id: x
                                .author_id
                                .with_scale(0)
                                .into_bigint_and_exponent()
                                .0
                                .to_u128()?,
                            edited_at: x.edited_at,
                            embeds: vec![],
                            author: User {
                                id: x.author.id,
                                name: x.author.name,
                                avatar: None,
                                guilds: None,
                                flags: UserFlags::from_bits_truncate(x.author.flags),
                                discriminator: x.author.discriminator,
                            },
                            nonce: None,
                        })
                    })
                    .collect(),
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("database returned a error: {}", e),
                    })
                }
            }
        }
    };

    HttpResponse::Ok().json(MessageHistory { messages })
}
