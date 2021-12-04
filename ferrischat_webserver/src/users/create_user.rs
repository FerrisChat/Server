use crate::WebServerError;
use axum::Json;
use ferrischat_common::request_json::UserCreateJson;
use ferrischat_common::types::{
    ErrorJson, ModelType, User, UserFlags,
};
use ferrischat_snowflake_generator::generate_snowflake;
use rand::Rng;
use serde::Serialize;
use tokio::sync::oneshot::channel;

/// POST /api/v0/users/
/// Creates a ferrischat user with the given info
pub async fn create_user(
    user_data: Json<UserCreateJson>,
) -> Result<crate::Json<User>, WebServerError> {
    let db = get_db_or_fail!();
    let node_id = get_node_id!();
    let user_id = generate_snowflake::<0>(ModelType::User as u8, node_id);
    let UserCreateJson {
        username,
        email,
        password,
        pronouns,
    } = user_data.0;
    // Gets a discriminator for the user
    let user_discrim = {
        // Makes sure the name doesn't already exist
        let existing: Vec<i16> =
            sqlx::query!("SELECT discriminator FROM users WHERE name = $1", username)
                .fetch_all(db)
                .await
                .map(|r| r.into_iter().map(|x| x.discriminator).collect())
                .map_err(|e| WebServerError::Database(e))?;
        // your discrim can be between 1 and 9999
        let available = (1..=9999)
            .filter(|x| !existing.contains(x))
            .collect::<Vec<_>>();
        *available
            .get(rand::thread_rng().gen_range(0..available.len()))
            .ok_or_else(
                (
                    409,
                    ErrorJson::new_409(
                        "This username has all possible discriminators taken.".to_string(),
                    ),
                )
                    .into,
            )?
    };
    // Hash the password for security.
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

    let db_pronouns = pronouns.map(|p| p as i16);
    let bigint_user_id = u128_to_bigdecimal!(user_id);
    // tell the database about our new user
    sqlx::query!(
        "INSERT INTO users VALUES ($1, $2, $3, $4, $5, $6, false, $7)",
        bigint_user_id,
        username,
        0,
        email,
        hashed_password,
        user_discrim,
        db_pronouns,
    )
    .execute(db)
    .await?;

    Ok(crate::Json {
        obj: User {
            id: user_id,
            name: username,
            avatar: None,
            guilds: None,
            flags: UserFlags::empty(),
            discriminator: user_discrim,
            pronouns,
        },
        code: 201,
    })
}
