-- Add migration script here
alter table messages alter column channel_id type numeric(39) using channel_id::numeric(39);