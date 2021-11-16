-- Add migration script here
ALTER TABLE users ADD COLUMN verified BOOLEAN DEFAULT false;
