-- Add migration script here
CREATE TABLE IF NOT EXISTS users (id BIGINT PRIMARY KEY, name varchar(100), guilds BIGINT);
CREATE TABLE IF NOT EXISTS guilds (id BIGINT PRIMARY KEY, owner_id BIGINT references users, name varchar(100), channels BIGINT, users BIGINT);
CREATE TABLE IF NOT EXISTS channels (id BIGINT PRIMARY KEY, name varchar(100));
CREATE TABLE IF NOT EXISTS members (user_id BIGINT references users, guild_id BIGINT references guilds);
CREATE TABLE IF NOT EXISTS messages (id BIGINT PRIMARY KEY, content varchar(100), channel BIGINT references channels, reactions BIGINT);