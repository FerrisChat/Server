-- Add migration script here
ALTER TABLE users ADD COLUMN discriminator SMALLINT;