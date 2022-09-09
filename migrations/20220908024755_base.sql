CREATE TYPE u128 AS (
    high BIGINT,
    low BIGINT
);

CREATE TABLE IF NOT EXISTS users (
    id u128 NOT NULL PRIMARY KEY,
    username TEXT NOT NULL,
    email TEXT NOT NULL,
    password TEXT NOT NULL,
    avatar TEXT,
    banner TEXT,
    bio TEXT,
    flags INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS tokens (
    user_id u128 NOT NULL,
    token TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id)
        REFERENCES users (id)
        ON DELETE CASCADE
);