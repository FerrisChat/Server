-- Add migration script here
alter table bots alter column owner_id type numeric(39) using owner_id::numeric(39);
alter table bots alter column user_id type numeric(39) using user_id::numeric(39);