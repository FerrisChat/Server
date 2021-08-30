-- Add migration script here
ALTER TABLE invites ALTER COLUMN uses SET NOT NULL;
