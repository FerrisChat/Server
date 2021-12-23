use crate::ws::fire_event;
use crate::WebServerError;
use axum::Json;
use ferrischat_common::request_json::GuildCreateJson;
use ferrischat_common::types::{Guild, GuildFlags, Member, ModelType, ErrorJson};
use ferrischat_common::ws::WsOutboundEvent;
use ferrischat_snowflake_generator::generate_snowflake;

/// POST /v0/guilds/
pub async fn create_guild(
    crate::Authorization(user_id, is_bot): crate::Authorization,
    guild_info: Json<GuildCreateJson>,
) -> Result<crate::Json<Guild>, WebServerError> {
    if is_bot {
        return Err(ErrorJson::new_403("Bots are not permitted to create guilds!".to_string()).into());
    };
    let db = get_db_or_fail!();
    let node_id = get_node_id!();
    let guild_id = generate_snowflake::<0>(ModelType::Guild as u8, node_id);
    let bigint_guild_id = u128_to_bigdecimal!(guild_id);
    let bigint_user_id = u128_to_bigdecimal!(user_id);
    let GuildCreateJson { name } = guild_info.0;

    sqlx::query!(
        "INSERT INTO guilds(id, owner_id, name, flags) VALUES ($1, $2, $3, $4)",
        bigint_guild_id,
        bigint_user_id,
        name,
        0
    )
    .execute(db)
    .await?;

    sqlx::query!(
        "INSERT INTO members VALUES ($1, $2)",
        bigint_user_id,
        bigint_guild_id
    )
    .execute(db)
    .await?;

    let guild_obj = Guild {
        id: guild_id,
        owner_id: user_id,
        name,
        channels: None,
        flags: GuildFlags::empty(),
        members: Some(vec![Member {
            guild_id: Some(guild_id),
            user_id: Some(user_id),
            user: None,
            guild: None,
        }]),
        roles: None,
        icon: None,
    };

    let event = WsOutboundEvent::GuildCreate {
        guild: guild_obj.clone(),
    };

    fire_event(&event).await?;

    Ok(crate::Json {
        obj: guild_obj,
        code: 201,
    })
}
