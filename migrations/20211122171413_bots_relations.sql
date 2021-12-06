-- Add migration script here
alter table bots
    add constraint bots_bot_id_fk
        foreign key (user_id) references users
            on delete cascade;

alter table bots
    add constraint bots_owner_id_fk
        foreign key (owner_id) references users
            on delete cascade;
