#[macro_export]
macro_rules! get_item_id {
    ($req:expr, $name:expr) => {{
        use actix_web::HttpResponse;
        match $req.match_info().get($name) {
            Some(id) => match id.parse() {
                Ok(id) => id,
                Err(e) => {
                    return HttpResponse::BadRequest().json(InternalServerErrorJson {
                        reason: format!("Failed to parse user ID as u128: {}", e),
                    })
                }
            },
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: "User ID not found in match_info".to_string(),
                })
            }
        }
    }};
}
