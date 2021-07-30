use rocket::http::Status;
use rocket::response::{content, status};

pub async fn create_channel() -> status::Custom<&'static str> {
    status::Custom(Status::Created, "created channel test")
}
