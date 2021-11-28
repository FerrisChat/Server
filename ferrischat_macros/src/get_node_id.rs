#[macro_export]
/// Expands to a block that either fetches the current node ID and returns it or
/// returns a HTTP 500 from the function.
macro_rules! get_node_id {
    () => {
        ferrischat_redis::NODE_ID
            .get()
            .map(|i| *i)
            .ok_or(WebServerError::MissingNodeId)?
    };
}
