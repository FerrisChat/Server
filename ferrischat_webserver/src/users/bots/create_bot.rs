use actix_web::{web::Json, HttpResponse, Responder};
use ferrischat_common::request_json::BotCreateJson;
use ferrischat_common::types::{
    InternalServerErrorJson, Json as MsgJson, ModelType, User, UserFlags,
};
use ferrischat_snowflake_generator::generate_snowflake;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tokio::sync::oneshot::channel;

/// POST /api/v0/users/{user_id}/bots
/// Creates a FerrisChat bot with the given info
pub async fn create_bot(
    auth: crate::Authorization,
    bot_data: Json<BotCreateJson>,
) -> impl Responder {
    let db = get_db_or_fail!();
    let node_id = get_node_id!();
    let user_id = generate_snowflake::<0>(ModelType::User as u8, node_id);
    let BotCreateJson { username } = bot_data.0;
    let email = format!("{}@bots.ferris.chat", user_id);
    let password: String = (&mut thread_rng())
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    // Gets a discriminator for the user
    let user_discrim = {
        // Makes sure the name doesn't already exist
        let existing: Vec<i16> =
            match sqlx::query!("SELECT discriminator FROM users WHERE name = $1", username)
                .fetch_all(db)
                .await
            {
                Ok(r) => r.into_iter().map(|x| x.discriminator).collect(),
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("DB returned a error: {}", e),
                        is_bug: false,
                        link: None,
                    })
                }
            };
        // your discrim can be between 1 and 9999
        let available = (1..=9999)
            .filter(|x| !existing.contains(x))
            .collect::<Vec<_>>();
        match available.get(rand::thread_rng().gen_range(0..available.len())) {
            Some(d) => *d,
            None => {
                return HttpResponse::Conflict().json(MsgJson {
                    message: "This username has all possible discriminators taken.".to_string(),
                })
            }
        }
    };
    let hashed_password = {
        let hasher = match ferrischat_auth::GLOBAL_HASHER.get() {
            Some(h) => h,
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "Password hasher not found".to_string(),
                    is_bug: true,
                    link: Option::from(
                        "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                            .to_string(),
                    ),
                })
            }
        };
        let (tx, rx) = channel();
        // if this fn errors it will be caught because tx will be dropped as well
        // resulting in a error in the match
        let _ = hasher.send((password, tx)).await;
        match rx.await {
            Ok(d) => match d {
                Ok(s) => s,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("Failed to hash password: {}", e),
                        is_bug: true,
                        link: Option::from(
                            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&\
                        labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+"
                                .to_string(),
                        ),
                    })
                }
            },
            Err(_) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "Other end hung up connection".to_string(),
                    is_bug: false,
                    link: None,
                })
            }
        }
    };
    let bigint_bot_id = u128_to_bigdecimal!(user_id);
    let bigint_owner_id = u128_to_bigdecimal!(auth.0);

    if let Err(e) = sqlx::query!(
        "INSERT INTO bots VALUES ($1, $2)",
        bigint_bot_id,
        bigint_owner_id
    )
    .execute(db)
    .await
    {
        return HttpResponse::InternalServerError().json(InternalServerErrorJson {
            reason: format!("DB returned a error: {}", e),
            is_bug: false,
            link: None,
        });
    }

    // tell the database about our new bot
    match sqlx::query!(
        "INSERT INTO users VALUES ($1, $2, $3, $4, $5, $6)",
        bigint_bot_id,
        username,
        UserFlags::BOT_ACCOUNT.bits(),
        email,
        hashed_password,
        user_discrim,
    )
    .execute(db)
    .await
    {
        Ok(_) => HttpResponse::Created().json(User {
            id: user_id,
            name: username,
            avatar: None,
            guilds: None,
            flags: UserFlags::BOT_ACCOUNT,
            discriminator: user_discrim,
        }),
        Err(e) => match e {
            sqlx::Error::Database(e) => {
                if e.code() == Some("23505".into()) {
                    HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: "A bot with this email already exists? (this is DEFINITELY a bug)"
                            .to_string(),
                        is_bug: true,
                        link: None,
                    })
                } else {
                    HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("DB returned a error: {}", e),
                        is_bug: false,
                        link: None,
                    })
                }
            }
            _ => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned a error: {}", e),
                is_bug: false,
                link: None,
            }),
        },
    }
}
