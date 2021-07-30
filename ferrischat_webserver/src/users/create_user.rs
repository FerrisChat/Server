use rocket::http::Status;
use rocket::response::{content, status};

pub async fn create_user() -> status::Custom<&'static str> {
    status::Custom(Status::Created, "created user test")
}
