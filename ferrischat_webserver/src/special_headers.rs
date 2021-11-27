use headers::{Error, Header, HeaderName, HeaderValue};
use std::iter::once;

// Email header

/// A header for emails in authentication.
pub(crate) struct Email(String);

impl Header for Email {
    fn name() -> &'static HeaderName {
        &HeaderName::from_static("Email")
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        Ok(Self(
            values
                .next()
                .ok_or_else(Error::invalid)?
                .to_str()
                .map_err(|_| Error::invalid())?,
        ))
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        values.extend(once(
            HeaderValue::from_str(&self.0)
                .expect("invalid header value found that should be impossible"),
        ));
    }
}

impl Email {
    pub fn into_inner(self) -> String {
        self.0
    }
}

// Password header

/// A header for passwords in authentication.
pub(crate) struct Password(String);

impl Header for Password {
    fn name() -> &'static HeaderName {
        &HeaderName::from_static("Password")
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        Ok(Self(
            values
                .next()
                .ok_or_else(Error::invalid)?
                .to_str()
                .map_err(|_| Error::invalid())?,
        ))
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        values.extend(once(
            HeaderValue::from_str(&self.0)
                .expect("invalid header value found that should be impossible"),
        ));
    }
}

impl Password {
    pub fn into_inner(self) -> String {
        self.0
    }
}
