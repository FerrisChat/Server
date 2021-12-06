pub enum VerifyTokenFailure {
    MissingDatabase,
    InvalidToken,
    DbError(sqlx::Error),
    VerifierError(argon2_async::Error),
}

impl From<sqlx::Error> for VerifyTokenFailure {
    #[inline]
    fn from(e: sqlx::Error) -> Self {
        Self::DbError(e)
    }
}

impl From<argon2_async::Error> for VerifyTokenFailure {
    fn from(e: argon2_async::Error) -> Self {
        Self::VerifierError(e)
    }
}

#[allow(clippy::missing_panics_doc)]
/// Verify a user's token.
///
/// # Errors
/// Returns an error if any of the following happen:
/// * The DB pool is not initialized.
/// * Auth data is invalid.
/// * The DB returns an error.
/// * The global verifier is not found.
/// * A verification error occurs.
pub async fn verify_token(user_id: u128, secret: String) -> Result<(), VerifyTokenFailure> {
    let id_bigint = u128_to_bigdecimal!(user_id);
    let db = ferrischat_db::DATABASE_POOL
        .get()
        .ok_or(VerifyTokenFailure::MissingDatabase)?;

    let db_token = sqlx::query!(
        "SELECT (auth_token) FROM auth_tokens WHERE user_id = $1",
        id_bigint
    )
    .fetch_optional(db)
    .await?
    .ok_or(VerifyTokenFailure::InvalidToken)?
    .auth_token;

    if argon2_async::verify(secret, db_token).await? {
        Ok(())
    } else {
        Err(VerifyTokenFailure::InvalidToken)
    }
}
