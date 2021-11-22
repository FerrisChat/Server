use actix_web::{web::Json, HttpResponse, Responder};
use ferrischat_common::request_json::UserCreateJson;
use ferrischat_common::types::{InternalServerErrorJson, ModelType, User, UserFlags};
use ferrischat_snowflake_generator::generate_snowflake;
use rand::Rng;
use tokio::sync::oneshot::channel;

/// POST /api/v0/users/
/// Creates a ferrischat user with the given info
pub async fn create_user(user_data: Json<UserCreateJson>) -> impl Responder {
    let db = get_db_or_fail!();
    let node_id = get_node_id!();
    let user_id = generate_snowflake::<0>(ModelType::User as u8, node_id);
    let UserCreateJson {
        username,
        email,
        password,
    } = user_data.0;
    // Gets a descriminator for the user
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
                    })
                }
            };
        // your descrim can be between 1 and 9999
        let available = (1..=9999)
            .filter(|x| !existing.contains(x))
            .collect::<Vec<_>>();
        match available.get(rand::thread_rng().gen_range(0..available.len())) {
            Some(d) => *d,
            None => {
                return HttpResponse::Conflict().json(InternalServerErrorJson {
                    reason: "This username has all possible discriminators taken.".to_string(),
                })
            }
        }
    };
    // Hash the password for security.
    let hashed_password = {
        let hasher = match ferrischat_auth::GLOBAL_HASHER.get() {
            Some(h) => h,
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "Password hasher not found".to_string(),
                })
            }
        };
        let (tx, rx) = channel();
        // if this fn errors it will be caught because tx will be dropped as well
        // resulting in a error in the match
        let _tx = hasher.send((password, tx)).await;
        match rx.await {
            Ok(d) => match d {
                Ok(s) => s,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("Failed to hash password: {}", e),
                    })
                }
            },
            Err(_) => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "Other end hung up connection".to_string(),
                })
            }
        }
    };

    let bigint_user_id = u128_to_bigdecimal!(user_id);
    // tell the database about our new user
    match sqlx::query!(
        "INSERT INTO users VALUES ($1, $2, $3, $4, $5, $6)",
        bigint_user_id,
        username,
        0,
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
            flags: UserFlags::empty(),
            discriminator: user_discrim,
        }),
        Err(e) => match e {
            sqlx::Error::Database(e) => {
                if e.code() == Some("23505".into()) {
                    HttpResponse::Conflict().json(InternalServerErrorJson {
                        reason: "A user with this email already exists".to_string(),
                    })
                } else {
                    HttpResponse::InternalServerError().json(InternalServerErrorJson {
                        reason: format!("DB returned a error: {}", e),
                    })
                }
            }
            _ => HttpResponse::InternalServerError().json(InternalServerErrorJson {
                reason: format!("DB returned a error: {}", e),
            }),
        },
    }
}
