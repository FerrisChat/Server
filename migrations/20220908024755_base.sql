CREATE TYPE u128 AS (
    high BIGINT,
    low BIGINT
);

CREATE OR REPLACE FUNCTION generate_discriminator(TEXT)
RETURNS SMALLINT
LANGUAGE plpgsql
AS $$
DECLARE
    out SMALLINT;
BEGIN
    SELECT * FROM (
        SELECT
            trunc(random() * 9998 + 1) AS discrim
        FROM
            generate_series(1, 9999)
    ) AS result
    WHERE result.discrim NOT IN (
        SELECT discriminator FROM users WHERE username = $1
    )
    LIMIT 1
    INTO out;
    RETURN out;
END;
$$;

CREATE TABLE IF NOT EXISTS users (
    id u128 NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    discriminator SMALLINT NOT NULL DEFAULT generate_discriminator('username'),
    email TEXT,
    password TEXT,
    avatar TEXT,
    banner TEXT,
    bio TEXT,
    flags INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tokens (
    user_id u128 NOT NULL,
    token TEXT NOT NULL,
    expires_at TIMESTAMP,
    FOREIGN KEY (user_id)
        REFERENCES users (id)
        ON DELETE CASCADE
);