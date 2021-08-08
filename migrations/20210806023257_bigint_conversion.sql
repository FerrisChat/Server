-- Add migration script here
alter table users alter column id type numeric(39) using id::numeric(39);
alter table messages alter column id type numeric(39) using id::numeric(39);
alter table guilds alter column id type numeric(39) using id::numeric(39);
alter table channels alter column id type numeric(39) using id::numeric(39);
