-- Add migration script here
ALTER TABLE guilds
    add column flags BIGINT;