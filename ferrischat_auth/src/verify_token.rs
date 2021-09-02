use tokio::sync::oneshot::channel;

pub enum VerifyTokenFailure {
    InternalServerError(String),
    Unauthorized(String),
}

/// Verify a user's token.
pub async fn verify_token(user_id: u128, secret: String) -> Result<(), VerifyTokenFailure> {
    let id_bigint = u128_to_bigdecimal!(user_id);
    let db = match ferrischat_db::DATABASE_POOL.get() {
        Some(db) => db,
        None => {
            return Err(VerifyTokenFailure::InternalServerError(
                "Database pool was not initialized".to_string(),
            ));
        }
    };

    let db_token = match sqlx::query!(
        "SELECT (auth_token) FROM auth_tokens WHERE user_id = $1",
        id_bigint
    )
    .fetch_optional(db)
    .await
    {
        Ok(t) => match t {
            Some(t) => t.auth_token,
            None => {
                return Err(VerifyTokenFailure::Unauthorized(
                    "`Authorization` header passed was invalid".to_string(),
                ));
            }
        },
        Err(e) => {
            return Err(VerifyTokenFailure::InternalServerError(format!(
                "Database returned a error: {}",
                e
            )));
        }
    };
    let verifier = match crate::GLOBAL_VERIFIER.get() {
        Some(v) => v,
        None => {
            return Err(VerifyTokenFailure::InternalServerError(
                "Global hash verifier not found".to_string(),
            ))
        }
    };
    let (tx, rx) = channel();
    // if the send failed, we'll know because the receiver we wait upon below will fail instantly
    let _ = verifier.send(((secret, db_token), tx)).await;
    let res = match rx.await {
        Ok(r) => match r {
            Ok(r) => r,
            Err(e) => {
                return Err(VerifyTokenFailure::InternalServerError(format!(
                    "Failed to verify token: {}",
                    e
                )))
            }
        },
        Err(_) => {
            return Err(VerifyTokenFailure::InternalServerError(
                "A impossible situation seems to have happened".to_string(),
            ))
        }
    };
    if res {
        Ok(())
    } else {
        // we specifically do not define the boundary between no token and
        // wrong tokens
        Err(VerifyTokenFailure::Unauthorized(
            "`Authorization` header passed was invalid".to_string(),
        ))
    }
}
