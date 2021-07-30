use rocket::http::Status;
use rocket::response::{content, status};

pub async fn get_channel(id: u64) -> status::Custom<&'static str> {
    status::Custom(Status::Ok, "found channel test")
}
