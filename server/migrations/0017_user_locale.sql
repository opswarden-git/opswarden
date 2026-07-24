alter table users
    add column if not exists locale text not null default 'en';

alter table users
    drop constraint if exists users_locale_supported;

alter table users
    add constraint users_locale_supported check (locale in ('en', 'fr'));
