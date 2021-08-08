#[macro_export]
/// Expands to a block that either fetches the DB pool and returns it or
/// returns a HTTP 500 from the function.
macro_rules! get_db_or_fail {
    () => {{
        use ferrischat_common::types::InternalServerErrorJson;
        use ferrischat_db::DATABASE_POOL;
        match DATABASE_POOL.get() {
            Some(db) => db,
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "Database pool was not found".to_string(),
                })
            }
        }
    }};
}
