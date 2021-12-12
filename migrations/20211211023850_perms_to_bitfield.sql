-- Add migration script here

DROP TABLE roles CASCADE;

CREATE TABLE IF NOT EXISTS roles
(
    id           numeric(39) PRIMARY KEY                         NOT NULL,
    name         VARCHAR(256)                                    NOT NULL,
    color        INT,
    position     SMALLINT                                        NOT NULL DEFAULT 0,
    permissions  bytea                                           NOT NULL DEFAULT '',
    parent_guild numeric(39) REFERENCES guilds ON DELETE CASCADE NOT NULL
);
CREATE TABLE IF NOT EXISTS role_data
(
    internal_id numeric(39) PRIMARY KEY                         NOT NULL,
    guild_id    numeric(39) REFERENCES guilds ON DELETE CASCADE NOT NULL,
    user_id     numeric(39) REFERENCES users ON DELETE CASCADE  NOT NULL,
    role_id     numeric(39) REFERENCES roles ON DELETE CASCADE  NOT NULL
);

