CREATE TABLE IF NOT EXISTS channels (
    id u128 NOT NULL PRIMARY KEY,
    guild_id u128,
    type TEXT NOT NULL,
    name TEXT,
    position SMALLINT,
    parent_id u128,
    topic TEXT,
    icon TEXT,
    slowmode INTEGER,
    nsfw BOOLEAN,
    locked BOOLEAN,
    user_limit SMALLINT,
    owner_id u128,
    FOREIGN KEY (guild_id)
        REFERENCES guilds(id)
        ON DELETE CASCADE,
    FOREIGN KEY (parent_id)
        REFERENCES channels(id)
        ON DELETE SET NULL,
    FOREIGN KEY (owner_id)
        REFERENCES users(id)
        ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS channel_overwrites (
    guild_id u128 NOT NULL,
    channel_id u128 NOT NULL,
    target_id u128 NOT NULL,
    allow BIGINT,
    deny BIGINT,
    PRIMARY KEY (guild_id, channel_id, target_id),
    FOREIGN KEY (guild_id)
        REFERENCES guilds (id)
        ON DELETE CASCADE,
    FOREIGN KEY (channel_id)
        REFERENCES channels (id)
        ON DELETE CASCADE,
    FOREIGN KEY (target_id)
        REFERENCES users (id)
        ON DELETE CASCADE,
    FOREIGN KEY (target_id)
        REFERENCES roles (id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS channel_recipients (
    channel_id u128 NOT NULL,
    user_id u128 NOT NULL,
    PRIMARY KEY (channel_id, user_id),
    FOREIGN KEY (channel_id)
        REFERENCES channels(id)
        ON DELETE CASCADE,
    FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

ALTER TABLE roles
DROP COLUMN IF EXISTS permissions,
ADD COLUMN IF NOT EXISTS allowed_permissions BIGINT DEFAULT 0,
ADD COLUMN IF NOT EXISTS denied_permissions BIGINT DEFAULT 0;