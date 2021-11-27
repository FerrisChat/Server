#[macro_export]
/// Expands to a block that either fetches the DB pool and returns it or
/// returns a HTTP 500 from the function.
macro_rules! get_db_or_fail {
    () => {
        ferrischat_db::DATABASE_POOL
            .get()
            .ok_or(crate::WebServerError::MissingDatabase)?
    };
}
