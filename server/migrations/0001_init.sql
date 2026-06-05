create table if not exists users (
    id uuid primary key,
    email text unique not null,
    password_hash text not null,
    created_at timestamptz not null default now()
);