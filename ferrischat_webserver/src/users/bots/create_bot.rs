use crate::WebServerError;
use axum::Json;
use ferrischat_common::request_json::BotCreateJson;
use ferrischat_common::types::{
    ErrorJson, ModelType, User, UserFlags,
};
use ferrischat_snowflake_generator::generate_snowflake;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Serialize;
use tokio::sync::oneshot::channel;

/// POST `/api/v0/users/{user_id}/bots`
/// Creates a `FerrisChat` bot with the given info
pub async fn create_bot(
    auth: crate::Authorization,
    Json(BotCreateJson { username }): Json<BotCreateJson>,
) -> Result<crate::Json<User>, WebServerError> {
    let db = get_db_or_fail!();
    let node_id = get_node_id!();
    let user_id = generate_snowflake::<0>(ModelType::User as u8, node_id);
    let email = format!("{}@bots.ferris.chat", user_id);
    let password = (&mut thread_rng())
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
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
            .ok_or_else(|| ErrorJson::new_409(
                "this username has all possible discriminators taken".to_string(),
            ),
            )?
    };
    let hashed_password = {
        let (tx, rx) = channel();
        ferrischat_auth::GLOBAL_HASHER
            .get()
            .ok_or(WebServerError::MissingHasher)?
            .send((password, tx))
            .await
            .map_err(|_| {
                (
                    500,
                    ErrorJson::new_500(
                        "Password hasher has hung up connection".to_string(),
                        false,
                        None,
                    ),
                )
                    .into()
            })?;
        rx.await
          .unwrap_or_else(|e| {
              unreachable!(
                  "failed to receive value from channel despite value being sent earlier on: {}",
                  e
              )
          })
          .map_err(|e| {
              (
                  500,
                  ErrorJson::new_500(
                      format!("failed to hash token: {}", e),
                      true,
                      Some(
                          "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                                         labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+failed+to+hash+token"
                              .to_string(),
                      ),
                  ),
              )
                  .into()
          })?
    };
    let bigint_bot_id = u128_to_bigdecimal!(user_id);
    let bigint_owner_id = u128_to_bigdecimal!(auth.0);

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
        },
        code: 201,
    })
}
