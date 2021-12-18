#[derive(Debug)]
pub enum UpdatePermissionsError {
    DbError(ferrischat_db::sqlx::Error),
}
