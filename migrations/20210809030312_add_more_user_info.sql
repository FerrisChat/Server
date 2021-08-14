-- Add migration script here
ALTER TABLE users ADD COLUMN email varchar(100);
ALTER TABLE users ADD COLUMN password varchar(100);