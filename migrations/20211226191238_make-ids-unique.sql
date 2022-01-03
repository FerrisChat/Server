-- Add migration script here
create unique index users_id_uindex
    on users (id);
create unique index bots_id_uindex
    on bots (user_id);
create unique index guilds_id_uindex
    on guilds (id);
create unique index channels_id_uindex
    on channels (id);
create unique index messages_id_uindex
    on messages (id);