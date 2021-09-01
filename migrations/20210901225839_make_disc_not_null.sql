-- Add migration script here
ALTER TABLE users ALTER COLUMN discriminator SET NOT NULL;