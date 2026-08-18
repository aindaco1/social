-- Durable provider publication state for assisted/API-gated providers such as TikTok.

CREATE TABLE IF NOT EXISTS provider_publications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    post_account_id INTEGER NOT NULL REFERENCES post_accounts(id) ON DELETE CASCADE,
    account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_account_id TEXT NOT NULL,
    mode TEXT NOT NULL,
    status TEXT NOT NULL,
    provider_status TEXT,
    provider_post_id TEXT,
    provider_url TEXT,
    upload_url TEXT,
    data_json TEXT,
    errors_json TEXT,
    status_checked_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (post_account_id)
);

CREATE INDEX IF NOT EXISTS provider_publications_provider_status_index
    ON provider_publications(provider, status, updated_at);
