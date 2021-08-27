use actix_web::web::Json;

use actix_web::{HttpRequest, HttpResponse, Responder};
use ferrischat_common::types::{InternalServerErrorJson, NotFoundJson, Channel};
use ferrischat_common::request_json::GuildUpdateJson;

use num_traits::ToPrimitive;

pub async fn edit_guild(req: HttpRequest, guild_info: Json<GuildUpdateJson>, _: crate::Authorization) -> impl Responder {
    let guild_id = get_item_id!(req, "guild_id");
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let GuildUpdateJson { name } = guild_info.0;

    let db = get_db_or_fail!();

    let resp = sqlx::query!("UPDATE guilds SET name = $2 WHERE id = $1 RETURNING *", bigint_guild_id, name)
        .fetch_optional(db)
        .await;
    
    match resp {
        Ok(resp) => match resp {
            Some(guild) => HttpResponse::Ok().json(Guild {
                id: guild.id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128(),
                owner_id: guild.owner_id
                    .with_scale(0)
                    .into_bigint_and_exponent()
                    .0
                    .to_u128(),
                name: guild.name.clone(),
                channels: None,
                members: None,
            }),
            None => HttpResponse::NotFound().json(NotFoundJson {
                message: "Guild not found".to_string(),
            }),
        Err(e) => HttpResponse::InternalServerError().json(InternalServerErrorJson {
            message: format!("DB returned an error: {}", e),
        },
        }
    }