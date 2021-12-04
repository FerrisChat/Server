#[macro_export]
/// Expands to a macro that either converts the `BigDecimal` passed in to a u128
/// or returns a HTTP 500 from the function.
macro_rules! bigdecimal_to_u128 {
    ($decimal:expr) => {{
        use ferrischat_common::types::ErrorJson;
        use num_traits::cast::ToPrimitive;
        $decimal
            .with_scale(0)
            .into_bigint_and_exponent()
            .0
            .to_u128().ok_or_else(|| ErrorJson::new_500("snowflake ID overflowed 128 bit integer".to_string(), true, Some(
            "https://github.com/FerrisChat/Server/issues/new?assignees=tazz4843&labels=bug&template=api_bug_report.yml&title=%5B500%5D%3A+snowflake+overflowed+u128"
                .to_string())))
            ?
    }};
}
