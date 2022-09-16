CREATE TABLE IF NOT EXISTS guilds (
    id u128 NOT NULL PRIMARY KEY,
    owner_id u128 NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    icon TEXT,
    banner TEXT,
    vanity_url TEXT,
    FOREIGN KEY (owner_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS members (
    id u128 NOT NULL,
    guild_id u128 NOT NULL,
    nick TEXT,
    joined_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id, guild_id),
    FOREIGN KEY (id)
        REFERENCES users(id)
        ON DELETE CASCADE,
    FOREIGN KEY (guild_id)
        REFERENCES guilds(id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS roles (
    id u128 NOT NULL PRIMARY KEY,
    guild_id u128 NOT NULL,
    name TEXT NOT NULL,
    -- Solid colors are stored as a single-element array of the color as 0xRRGGBB
    -- Gradients are stored as an array of 0xRRGGBBPP, where PP is the position in the gradient
    -- between 0 and 100.
    -- For gradient colors, due to the fact that they can overflow, the color is stored as a
    -- negative number, meaning that the colors should be read by bits/bitwise operators.
    color INTEGER[],
    gradient BOOLEAN NOT NULL DEFAULT FALSE,
    permissions BIGINT NOT NULL DEFAULT 0,
    flags INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (guild_id)
        REFERENCES guilds(id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS role_data (
    role_id u128 NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    user_id u128 NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    guild_id u128 NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, user_id, guild_id)
);
