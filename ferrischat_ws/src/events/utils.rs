use super::error::WebSocketHandlerError;
use num_traits::ToPrimitive;
use sqlx::types::BigDecimal;

#[allow(dead_code, clippy::needless_pass_by_value)]
#[inline]
pub fn bigdecimal_to_u128(num: BigDecimal) -> Result<u128, WebSocketHandlerError> {
    match num.with_scale(0).into_bigint_and_exponent().0.to_u128() {
        Some(n) => Ok(n),
        None => Err(WebSocketHandlerError::BigDecimalToU128Fail),
    }
}
