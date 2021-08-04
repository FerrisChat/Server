#[macro_export]
macro_rules! get_db_or_fail {
    () => {{
        use ferrischat_db::DATABASE_POOL;
        match DATABASE_POOL.get() {
            Some(db) => db,
            None => return HttpResponse::InternalServerError().body("Database pool was not found"),
        }
    }};
}
