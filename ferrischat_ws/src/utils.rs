use sqlx::{Pool, Postgres};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

pub fn get_db_err<'t>() -> Result<&Pool<Postgres>, CloseFrame<'t>> {
    match ferrischat_db::DATABASE_POOL.get() {
        Some(db) => Ok(db),
        None => Err(CloseFrame {
            code: CloseCode::from(5003),
            reason: "Database pool not found".into(),
        }),
    }
}
