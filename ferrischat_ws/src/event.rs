#[derive(Debug, Clone)]
pub struct RedisMessage {
    pub(crate) channel: String,
    pub(crate) message: String,
}
