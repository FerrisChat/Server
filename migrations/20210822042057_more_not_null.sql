-- Add migration script here
ALTER TABLE channels ALTER COLUMN guild_id SET NOT NULL;

ALTER TABLE messages ALTER COLUMN author_id SET NOT NULL;

ALTER TABLE users DROP COLUMN guilds;
ALTER TABLE users ALTER COLUMN flags SET NOT NULL;