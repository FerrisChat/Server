use crate::WebServerError;
use axum::Json;
use ferrischat_common::request_json::BotCreateJson;
use ferrischat_common::types::{ErrorJson, ModelType, User, UserFlags};
use ferrischat_snowflake_generator::generate_snowflake;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

/// POST `/v0/users/me/bots`
/// Creates a `FerrisChat` bot with the given info
pub async fn create_bot(
    crate::Authorization(owner_id, is_bot): crate::Authorization,
    Json(BotCreateJson { username }): Json<BotCreateJson>,
) -> Result<crate::Json<User>, WebServerError> {
    if is_bot {
        return Err(ErrorJson::new_403("Bots cannot create/own bots!".to_string()).into());
    }
    let db = get_db_or_fail!();
    let bigint_owner_id = u128_to_bigdecimal!(owner_id);
    let node_id = get_node_id!();
    let user_id = generate_snowflake::<0>(ModelType::Bot as u8, node_id);
    let email = format!("{}@bots.ferris.chat", user_id);
    let password = (&mut thread_rng())
        .sample_iter(&Alphanumeric)
        .take(64)
        .collect::<Vec<u8>>();
    // Gets a discriminator for the user
    let user_discrim = {
        // Makes sure the name doesn't already exist
        let existing: Vec<i16> =
            sqlx::query!("SELECT discriminator FROM users WHERE name = $1", username)
                .fetch_all(db)
                .await?
                .into_iter()
                .map(|x| x.discriminator)
                .collect();
        // your discrim can be between 1 and 9999
        let available = (1..=9999)
            .filter(|x| !existing.contains(x))
            .collect::<Vec<_>>();
        *available
            .get(rand::thread_rng().gen_range(0..available.len()))
            .ok_or_else(|| {
                ErrorJson::new_409(
                    "this username has all possible discriminators taken".to_string(),
                )
            })?
    };
    let hashed_password = ferrischat_auth::hash(password).await?;
    let bigint_bot_id = u128_to_bigdecimal!(user_id);

    sqlx::query!(
        "INSERT INTO users VALUES ($1, $2, $3, $4, $5, $6)",
        bigint_bot_id,
        username,
        UserFlags::BOT_ACCOUNT.bits(),
        email,
        hashed_password,
        user_discrim,
    )
    .execute(db)
    .await?;

    sqlx::query!(
        "INSERT INTO bots VALUES ($1, $2)",
        bigint_bot_id,
        bigint_owner_id
    )
    .execute(db)
    .await?;

    Ok(crate::Json {
        obj: User {
            id: user_id,
            name: username,
            avatar: None,
            guilds: None,
            flags: UserFlags::BOT_ACCOUNT,
            discriminator: user_discrim,
            pronouns: None,
            is_bot
        },
        code: 201,
    })
}
