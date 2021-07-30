use rocket::http::Status;
use rocket::response::{content, status};

pub async fn delete_message(id: u64) -> status::Custom<&'static str> {
    status::Custom(Status::Ok, "deleted message test")
}
