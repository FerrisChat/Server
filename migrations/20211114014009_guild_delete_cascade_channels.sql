-- Add migration script here
alter table channels
    add constraint channels_guilds_fkey
        foreign key (guild_id) references guilds
            on delete cascade;
