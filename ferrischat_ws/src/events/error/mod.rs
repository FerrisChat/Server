mod from_impls;

pub enum WebSocketHandlerError {
    Database(sqlx::Error),
    TooManyIdentify,
    TokenVerifyFail(ferrischat_auth::VerifyTokenFailure),
    BigDecimalToU128Fail,
    ConnectionClosing,
}
