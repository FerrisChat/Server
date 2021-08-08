-- Add migration script here
ALTER TABLE guilds ALTER COLUMN owner_id TYPE numeric(39);