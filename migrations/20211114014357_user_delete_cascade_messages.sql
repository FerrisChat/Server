-- Add migration script here
alter table messages drop constraint messages_author_id_fkey;

alter table messages
    add constraint messages_author_id_fkey
        foreign key (author_id) references users
            on delete cascade;
