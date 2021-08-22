-- Add migration script here
ALTER TABLE messages ADD COLUMN author_id numeric(39) REFERENCES users;