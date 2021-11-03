-- Add migration script here
CREATE TABLE IF NOT EXISTS roles (id numeric(39) PRIMARY KEY, name VARCHAR(256), color INT, position SMALLINT, parent_guild numeric(39) REFERENCES guilds ON DELETE CASCADE);
CREATE TABLE IF NOT EXISTS role_data (internal_id numeric(39) PRIMARY KEY, guild_id numeric(39) REFERENCES guilds ON DELETE CASCADE, user_id numeric(39) REFERENCES members ON DELETE CASCADE, role_id numeric(39) REFERENCES roles ON DELETE CASCADE);
