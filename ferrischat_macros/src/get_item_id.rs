#[macro_export]
macro_rules! get_item_id {
    ($req:expr, $name:expr) => {{
        use actix_web::HttpResponse;
        match $req.match_info().get($name) {
            Some(id) => match id.parse::<u128>() {
                Ok(id) => id,
                Err(e) => {
                    return HttpResponse::BadRequest().json(InternalServerErrorJson {
                        reason: format!(stringify!("Failed to parse " $name " as u128: {}"), e),
                    })
                }
            },
            None => {
                return HttpResponse::InternalServerError().json(InternalServerErrorJson {
                    reason: stringify!($name " not found in match_info: this is a bug, please report it at \
                    https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&labels=bug&\
                    template=api_bug_report.yml&title=%5B500%5D%3A+" $name "+not+found+in+match_info").to_string(),
                })
            }
        }
    }};
}
