use http::{header, HeaderMap, HeaderValue, StatusCode};

#[allow(clippy::unused_async)]
/// Always returns HTTP 501 with a Retry-After date of 1 April 2022 at 00:00:00+00:00
pub async fn not_implemented() -> (HeaderMap, StatusCode) {
    let mut map = HeaderMap::with_capacity(1);
    map.insert(
        header::RETRY_AFTER,
        HeaderValue::from_static("Fri, 01 Apr 2022 00:00:00 GMT"),
    );

    (map, StatusCode::NOT_IMPLEMENTED)
}
