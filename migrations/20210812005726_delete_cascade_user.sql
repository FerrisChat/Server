-- Add migration script here
alter table guilds drop constraint guilds_owner_id_fkey;

alter table guilds
    add constraint guilds_owner_id_fkey
        foreign key (owner_id) references users
            on delete cascade;


alter table members drop constraint members_user_id_fkey;

alter table members
    add constraint members_user_id_fkey
        foreign key (user_id) references users
            on delete cascade;


alter table messages drop constraint messages_channel_fkey;

alter table messages
    add constraint messages_channel_fkey
        foreign key (channel_id) references channels
            on delete cascade;
