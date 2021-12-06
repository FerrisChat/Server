-- Add migration script here
CREATE INDEX guilds_idx ON guilds (id, owner_id);
CREATE INDEX users_idx ON users (id, email);
CREATE INDEX auth_tokens_idx ON auth_tokens (user_id, auth_token);
CREATE INDEX members_idx ON members (user_id, guild_id);
CREATE INDEX role_data_idx ON role_data (guild_id, user_id, role_id);
CREATE INDEX channels_idx ON channels (id, guild_id);
CREATE INDEX messages_idx ON messages (id);
CREATE INDEX invites_idx ON invites (code);