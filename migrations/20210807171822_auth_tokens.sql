CREATE TABLE auth_tokens (
    user_id DECIMAL(39) REFERENCES users NOT NULL UNIQUE,
    auth_token TEXT NOT NULL
);