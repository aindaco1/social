-- Dust Wave Social local schema.
-- This preserves the Mixpost Lite data model and adds durable desktop job state.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS services (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    configuration_secret_ref TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 0 CHECK (active IN (0, 1))
);

CREATE TABLE IF NOT EXISTS accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    username TEXT,
    avatar_disk TEXT,
    avatar_path TEXT,
    provider TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    data_json TEXT,
    authorized INTEGER NOT NULL DEFAULT 0 CHECK (authorized IN (0, 1)),
    access_token_secret_ref TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (provider, provider_id)
);

CREATE TABLE IF NOT EXISTS posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    status INTEGER NOT NULL DEFAULT 0 CHECK (status IN (0, 1, 2, 3)),
    schedule_status INTEGER NOT NULL DEFAULT 0 CHECK (schedule_status IN (0, 1, 2)),
    scheduled_at TEXT,
    published_at TEXT,
    deleted_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS post_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider_post_id TEXT,
    data_json TEXT,
    errors_json TEXT,
    UNIQUE (post_id, account_id)
);

CREATE TABLE IF NOT EXISTS post_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    account_id INTEGER NOT NULL,
    is_original INTEGER NOT NULL DEFAULT 0 CHECK (is_original IN (0, 1)),
    content_json TEXT
);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    hex_color TEXT NOT NULL CHECK (length(hex_color) = 6),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tag_post (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    UNIQUE (tag_id, post_id)
);

CREATE TABLE IF NOT EXISTS media (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    disk TEXT NOT NULL,
    path TEXT NOT NULL,
    data_json TEXT,
    size INTEGER NOT NULL,
    size_total INTEGER NOT NULL,
    conversions_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    payload_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS imported_posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider_post_id TEXT NOT NULL,
    content_json TEXT NOT NULL,
    metrics_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (account_id, provider_post_id)
);

CREATE TABLE IF NOT EXISTS facebook_insights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    type INTEGER NOT NULL,
    value INTEGER NOT NULL,
    date TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (account_id, type, date)
);

CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    data_json TEXT NOT NULL,
    date TEXT NOT NULL,
    UNIQUE (account_id, date)
);

CREATE TABLE IF NOT EXISTS audience (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    total INTEGER NOT NULL DEFAULT 0,
    date TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS audience_entry_index ON audience(account_id, date);

CREATE TABLE IF NOT EXISTS job_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    run_at TEXT NOT NULL,
    locked_at TEXT,
    completed_at TEXT,
    failed_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS job_queue_status_run_at_index ON job_queue(status, run_at);

CREATE TABLE IF NOT EXISTS rate_limits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scope TEXT NOT NULL UNIQUE,
    retry_after_at TEXT NOT NULL,
    payload_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
