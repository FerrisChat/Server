-- Add migration script here
DROP TABLE IF EXISTS invites;
CREATE TABLE IF NOT EXISTS invites (
                                       code TEXT NOT NULL PRIMARY KEY,
                                       owner_id numeric(39) NOT NULL REFERENCES users ON DELETE CASCADE,
                                       guild_id numeric(39) NOT NULL REFERENCES guilds ON DELETE CASCADE,
                                       created_at TIMESTAMP NOT NULL,
                                       uses SMALLINT DEFAULT 0,
                                       max_uses SMALLINT DEFAULT null,
                                       max_age BIGINT DEFAULT null
);