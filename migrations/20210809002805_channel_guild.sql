-- Add migration script here
ALTER TABLE channels ADD COLUMN guild_id BIGINT;
ALTER TABLE messages RENAME COLUMN channel TO channel_id;