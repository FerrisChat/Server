-- Add migration script here
CREATE UNIQUE INDEX invites_code_uindex
    ON invites (code);
