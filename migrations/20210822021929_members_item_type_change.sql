-- Add migration script here
ALTER TABLE members ALTER COLUMN guild_id TYPE numeric(39);
ALTER TABLE members ALTER COLUMN user_id TYPE numeric(39);