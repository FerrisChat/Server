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

pub fn split_token(token: String) -> Result<(u128, String), SplitTokenError> {
    let mut auth = token.split('.');
    let id = match auth.next() {
        Some(id) => match match base64::decode_config(id, base64::URL_SAFE) {
            Ok(t) => match String::from_utf8(t) {
                Ok(s) => s.parse::<u128>(),
                Err(e) => return Err(SplitTokenError::InvalidUtf8(e)),
            },
            Err(e) => return Err(SplitTokenError::Base64DecodeError(e)),
        } {
            Ok(id) => id,
            Err(e) => return Err(SplitTokenError::InvalidInteger(e)),
        },
        None => return Err(SplitTokenError::MissingParts(0)),
    };
    let token = match auth.next() {
        Some(token) => token.to_string(),
        None => return Err(SplitTokenError::MissingParts(1)),
    };

    Ok((id, token))
}
