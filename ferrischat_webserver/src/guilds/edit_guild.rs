use crate::ws::fire_event;
use crate::WebServerError;
use axum::extract::Path;
use axum::Json;
use ferrischat_common::request_json::GuildUpdateJson;
use ferrischat_common::types::{ErrorJson, Guild, GuildFlags};
use ferrischat_common::ws::WsOutboundEvent;

pub async fn edit_guild(
    Path(guild_id): Path<u128>,
    Json(GuildUpdateJson { name }): Json<GuildUpdateJson>,
    _: crate::Authorization,
) -> Result<crate::Json<Guild>, WebServerError> {
    let db = get_db_or_fail!();

    let bigint_guild_id = u128_to_bigdecimal!(guild_id);

    let guild = sqlx::query!("SELECT * FROM guilds WHERE id = $1", bigint_guild_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown guild with ID {}", guild_id)))?;
    let old_guild_obj = Guild {
        id: bigdecimal_to_u128!(guild.id),
        owner_id: bigdecimal_to_u128!(guild.owner_id),
        name: guild.name,
        flags: GuildFlags::empty(),
        channels: None,
        members: None,
        roles: None,
    };

    if let Some(name) = name {
        sqlx::query!(
            "UPDATE guilds SET name = $1 WHERE id = $2",
            name,
            bigint_guild_id
        )
        .execute(db)
        .await?;
    }

    let guild = sqlx::query!("SELECT * FROM guilds WHERE id = $1", bigint_guild_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| ErrorJson::new_404(format!("Unknown guild with ID {}", guild_id)))?;
    let new_guild_obj = Guild {
        id: bigdecimal_to_u128!(guild.id),
        owner_id: bigdecimal_to_u128!(guild.owner_id),
        name: guild.name,
        flags: GuildFlags::empty(),
        channels: None,
        members: None,
        roles: None,
    };

    // TODO: impl Eq for all types
    // if old_guild_obj == new_guild_obj {}

    let event = WsOutboundEvent::GuildUpdate {
        old: old_guild_obj,
        new: new_guild_obj.clone(),
    };

    fire_event(format!("guild_{}", guild_id), &event).await?;
    Ok(crate::Json {
        obj: new_guild_obj,
        code: 200,
    })
}
