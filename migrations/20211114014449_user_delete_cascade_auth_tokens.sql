-- Add migration script here
alter table auth_tokens drop constraint auth_tokens_user_id_fkey;

alter table auth_tokens
    add constraint auth_tokens_user_id_fkey
        foreign key (user_id) references users
            on delete cascade;
