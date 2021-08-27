-- Add migration script here
CREATE TABLE IF NOT EXISTS invites (
    owner_id numeric(39) REFERENCES users ON DELETE CASCADE,
    guild_id numeric(39) REFERENCES guilds ON DELETE CASCADE,
    code TEXT,
    max_uses SMALLINT,
    max_age INT
);
