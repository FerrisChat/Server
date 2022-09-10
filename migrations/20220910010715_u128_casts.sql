CREATE OR REPLACE FUNCTION numeric_to_bits(NUMERIC)
RETURNS BIT(128)
LANGUAGE plpgsql
AS $$
DECLARE
    num ALIAS FOR $1;
    max_int NUMERIC := '9223372036854775808'::NUMERIC(19, 0);
    result BIT VARYING;
BEGIN
    WITH
        chunks(exponent, chunk) AS (
            SELECT
                exponent,
                floor((num / (max_int ^ exponent)::NUMERIC(300, 200)) % max_int)::BIGINT
            FROM generate_series(0, 3) exponent
        )
    SELECT bit_or(chunk::BIT(128) << (63 * exponent))
    FROM chunks INTO result;
    RETURN result;
END;
$$;

CREATE OR REPLACE FUNCTION u128_from_text(text TEXT)
RETURNS u128
LANGUAGE plpgsql
AS $$
DECLARE
    num NUMERIC(39);
    bits BIT(128);
    high BIGINT;
    low BIGINT;
BEGIN
    SELECT text::NUMERIC(39) INTO num;
    SELECT numeric_to_bits(num) INTO bits;
    SELECT bits::BIT(64)::BIGINT INTO high;
    SELECT (bits << 64)::BIT(64)::BIGINT INTO low;

    RETURN ROW(high, low);
END;
$$;

CREATE CAST (TEXT AS u128) WITH FUNCTION u128_from_text AS IMPLICIT;

CREATE OR REPLACE FUNCTION bits_to_numeric(bits BIT(128))
RETURNS NUMERIC(39)
AS $$
    SELECT sum(parts) FROM (
        SELECT get_bit(bits, 128 - n)::NUMERIC(39) * pow(2::NUMERIC(39), n - 1) AS parts
        FROM generate_series(1, length(bits), 1) g (n)
    ) AS sub
$$
LANGUAGE SQL
IMMUTABLE;

CREATE OR REPLACE FUNCTION u128_to_text(u128 u128)
RETURNS TEXT
LANGUAGE plpgsql
AS $$
BEGIN
    RETURN trunc(bits_to_numeric((u128.high::BIT(128) << 64) | (u128.low::BIT(64)::BIT(128) >> 64)))::TEXT;
END
$$;

CREATE CAST (u128 AS TEXT) WITH FUNCTION u128_to_text;