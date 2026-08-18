CREATE TABLE IF NOT EXISTS oauth_states (
    state TEXT PRIMARY KEY,
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at TEXT
);

CREATE TABLE IF NOT EXISTS tiktok_accounts (
    open_id TEXT PRIMARY KEY,
    union_id TEXT,
    display_name TEXT,
    username TEXT,
    avatar_url TEXT,
    profile_deep_link TEXT,
    scopes TEXT NOT NULL,
    access_token_ciphertext TEXT NOT NULL,
    refresh_token_ciphertext TEXT,
    access_token_expires_at TEXT NOT NULL,
    refresh_token_expires_at TEXT,
    token_type TEXT,
    last_imported_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS broker_connections (
    credential_hash TEXT PRIMARY KEY,
    open_id TEXT NOT NULL REFERENCES tiktok_accounts(open_id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    revoked_at TEXT
);

CREATE INDEX IF NOT EXISTS broker_connections_open_id_idx
    ON broker_connections (open_id);

CREATE INDEX IF NOT EXISTS oauth_states_expires_at_idx
    ON oauth_states (expires_at);
