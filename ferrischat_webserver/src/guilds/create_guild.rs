use actix_web::web::Json;
use actix_web::{HttpResponse, Responder};
use ferrischat_common::request_json::GuildCreateJson;
use ferrischat_common::types::{Guild, InternalServerErrorJson, Member, ModelType};
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /api/v0/guilds/
pub async fn create_guild(
    auth: crate::Authorization,
    guild_info: Json<GuildCreateJson>,
) -> impl Responder {
    let db = get_db_or_fail!();
    let node_id = get_node_id!();
    let guild_id = generate_snowflake::<0>(ModelType::Guild as u8, node_id);
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let bigint_user_id = u128_to_bigdecimal!(auth.0);
    let GuildCreateJson { name } = guild_info.0;
    if let Err(e) = sqlx::query!(
        "INSERT INTO guilds VALUES ($1, $2, $3)",
        bigint_guild_id,
        bigint_user_id,
        name
    )
    .execute(db)
    .await
    {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        });
    }

    if let Err(e) = sqlx::query!(
        "INSERT INTO members VALUES ($1, $2)",
        bigint_user_id,
        bigint_guild_id
    )
    .execute(db)
    .await
    {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
        });
    }

    HttpResponse::Created().json(Guild {
        id: guild_id,
        owner_id: auth.0,
        name,
        channels: None,
        members: Some(vec![Member {
            guild_id: Some(guild_id),
            user_id: Some(auth.0),
            user: None,
            guild: None,
        }]),
    })
}
