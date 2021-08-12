-- Add migration script here
create unique index users_email_uindex
    on users (email);
