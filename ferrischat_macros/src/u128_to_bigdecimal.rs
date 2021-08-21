#[macro_export]
macro_rules! u128_to_bigdecimal {
    ($input:expr) => {{
        use num_bigint::BigInt;
        use sqlx::types::BigDecimal;
        BigDecimal::new(BigInt::from($input), 0)
    }};
}
