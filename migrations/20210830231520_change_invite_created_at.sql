-- Add migration script --
drop table if exists invites;
CREATE TABLE IF NOT EXISTS invites (
                                       code TEXT NOT NULL PRIMARY KEY,
                                       owner_id numeric(39) NOT NULL REFERENCES users ON DELETE CASCADE,
                                       guild_id numeric(39) NOT NULL REFERENCES guilds ON DELETE CASCADE,
                                       created_at BIGINT NOT NULL,
                                       uses INT DEFAULT 0,
                                       max_uses SMALLINT DEFAULT null,
                                       max_age BIGINT DEFAULT null
);
ALTER TABLE invites ALTER COLUMN uses SET NOT NULL;
CREATE UNIQUE INDEX invites_code_uindex
    ON invites (code);
