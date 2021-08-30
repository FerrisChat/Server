-- Add migration script here
ALTER TABLE messages ADD COLUMN edited_at TIMESTAMP WITHOUT TIME ZONE;