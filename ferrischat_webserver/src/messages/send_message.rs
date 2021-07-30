use rocket::http::Status;
use rocket::response::{content, status};

pub async fn send_message(id: u64) -> status::Custom<&'static str> {
    status::Custom(Status::Created, "Send message test")
}
