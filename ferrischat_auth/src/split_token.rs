use base64::DecodeError;
use std::string::FromUtf8Error;

/// Errors returned when splitting a token into its constituent parts.
pub enum SplitTokenError {
    /// Invalid UTF-8 detected
    InvalidUtf8(FromUtf8Error),
    /// Invalid base64 encoded data detected
    Base64DecodeError(DecodeError),
    /// Invalid integer found in the base64 encoded data.
    InvalidInteger(std::num::ParseIntError),
    /// Parts of the token are missing.
    ///
    /// The attached integer shows what part is missing. Zero-indexed.
    MissingParts(u8),
}

/// Splits a token into its constituent parts and returns it.
///
/// # Errors
/// Returns an error if any of the following happen:
/// * Invalid UTF-8 is detected
/// * The base64 encoded data cannot be decoded
/// * A invalid integer is detected in the data
/// * Parts of the token are missing
pub fn split_token(token: &str) -> Result<(u128, String), SplitTokenError> {
    let mut auth = token.split('.');

    let id = String::from_utf8(
        base64::decode_config(
            auth.next().ok_or(SplitTokenError::MissingParts(0))?,
            base64::URL_SAFE,
        )
        .map_err(SplitTokenError::Base64DecodeError)?,
    )
    .map_err(SplitTokenError::InvalidUtf8)?
    .parse()
    .map_err(SplitTokenError::InvalidInteger)?;

    let token = auth
        .next()
        .ok_or(SplitTokenError::MissingParts(1))?
        .to_string();

    Ok((id, token))
}
