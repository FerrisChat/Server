#[macro_export]
/// Expands to a format! call that returns the path passed in with /api/v{} prefixed and
/// v{} replaced with the current API version, 0 as of this writing.
macro_rules! expand_version {
    ($path:expr) => {{
        use crate::API_VERSION;
        format!("/api/v{}/{}", API_VERSION, $path).as_str()
    }};
}
