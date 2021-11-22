use actix_web::{HttpResponse, Responder};

#[allow(clippy::unused_async)]
/// Always returns HTTP 501 with a Retry-After date of 1 April 2022 at 00:00:00+00:00
pub async fn not_implemented() -> impl Responder {
    HttpResponse::NotImplemented()
        .append_header(("Retry-After", "Fri, 01 Apr 2022 00:00:00 GMT"))
        .finish()
}
