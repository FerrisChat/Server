-- Add migration script here
alter table members drop constraint members_guild_id_fkey;

alter table members
    add constraint members_guild_id_fkey
        foreign key (guild_id) references guilds
            on delete cascade;
