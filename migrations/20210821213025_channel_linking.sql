-- Add migration script here
ALTER TABLE channels ALTER COLUMN guild_id TYPE numeric(39);
ALTER TABLE guilds DROP COLUMN channels;
ALTER TABLE guilds DROP COLUMN users;