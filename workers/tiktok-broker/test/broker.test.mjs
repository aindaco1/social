import assert from 'node:assert/strict';
import { test } from 'node:test';
import worker from '../src/index.js';

const tokenEncryptionKey = Buffer.from('0123456789abcdef0123456789abcdef').toString('base64');

test('OAuth callback stores encrypted TikTok tokens and issues a desktop credential', async (t) => {
    const env = testEnv();
    const fetchMock = installTikTokFetchMock(t);
    const oauth = await completeOAuth(env);

    assert.equal(oauth.provider, 'tiktok');
    assert.equal(oauth.provider_id, 'open_123');
    assert.equal(oauth.name, 'Dust Wave TikTok');
    assert.match(oauth.broker_connection, /^dw_tiktok_/);
    assert.deepEqual(oauth.scopes, ['user.info.basic', 'user.info.stats', 'video.list']);

    const account = env.DB.accounts.get('open_123');

    assert.ok(account);
    assert.match(account.access_token_ciphertext, /^v1\./);
    assert.notEqual(account.access_token_ciphertext, 'access-token-1');
    assert.match(account.refresh_token_ciphertext, /^v1\./);
    assert.equal(env.DB.connections.size, 1);
    assert.equal(fetchMock.calls.filter((call) => call.url.includes('/oauth/token/')).length, 1);
});

test('Analytics endpoint returns user stats and video metrics for a valid broker credential', async (t) => {
    const env = testEnv();
    installTikTokFetchMock(t);
    const oauth = await completeOAuth(env);

    const response = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/analytics',
        { headers: { Authorization: `Bearer ${oauth.broker_connection}` } },
    ), env);
    const payload = await response.json();
    const connection = [...env.DB.connections.values()][0];

    assert.equal(response.status, 200);
    assert.equal(payload.provider, 'tiktok');
    assert.equal(payload.provider_id, 'open_123');
    assert.equal(payload.user.follower_count, 4312);
    assert.equal(payload.videos.length, 2);
    assert.equal(payload.videos[0].view_count, 1200);
    assert.ok(connection.last_used_at);
});

test('Analytics endpoint rejects missing, invalid, and revoked broker credentials', async (t) => {
    const env = testEnv();
    const originalConsoleError = console.error;

    console.error = () => {};
    t.after(() => {
        console.error = originalConsoleError;
    });
    installTikTokFetchMock(t);
    const oauth = await completeOAuth(env);

    const missing = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/analytics',
    ), env);
    assert.equal(missing.status, 401);
    assert.equal((await missing.json()).error, 'missing_bearer_token');

    const invalid = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/analytics',
        { headers: { Authorization: 'Bearer wrong' } },
    ), env);
    assert.equal(invalid.status, 401);
    assert.equal((await invalid.json()).error, 'broker_connection_not_authorized');

    const connection = [...env.DB.connections.values()][0];

    connection.revoked_at = new Date().toISOString();
    const revoked = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/analytics',
        { headers: { Authorization: `Bearer ${oauth.broker_connection}` } },
    ), env);

    assert.equal(revoked.status, 401);
    assert.equal((await revoked.json()).error, 'broker_connection_not_authorized');
});

test('Analytics omits video list fetch when OAuth did not grant video.list', async (t) => {
    const env = testEnv({ scopes: 'user.info.basic,user.info.stats' });
    const fetchMock = installTikTokFetchMock(t, {
        tokenScope: 'user.info.basic,user.info.stats',
    });
    const oauth = await completeOAuth(env);

    const response = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/analytics',
        { headers: { Authorization: `Bearer ${oauth.broker_connection}` } },
    ), env);
    const payload = await response.json();

    assert.equal(response.status, 200);
    assert.deepEqual(payload.videos, []);
    assert.equal(fetchMock.calls.some((call) => call.url.includes('/video/list/')), false);
});

test('Admin status and revocation endpoints require the broker admin token', async (t) => {
    const env = testEnv();
    installTikTokFetchMock(t);
    const oauth = await completeOAuth(env);

    const unauthorized = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/status',
    ), env);
    assert.equal(unauthorized.status, 401);
    assert.equal((await unauthorized.json()).error, 'missing_bearer_token');

    const wrong = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/status',
        { headers: { Authorization: 'Bearer wrong-admin-token' } },
    ), env);
    assert.equal(wrong.status, 401);
    assert.equal((await wrong.json()).error, 'admin_not_authorized');

    const status = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/status',
        { headers: { Authorization: 'Bearer admin-secret' } },
    ), env);
    const statusPayload = await status.json();

    assert.equal(status.status, 200);
    assert.equal(statusPayload.provider_id, 'open_123');
    assert.equal(statusPayload.connections.total, 1);
    assert.equal(statusPayload.connections.active, 1);
    assert.equal(statusPayload.connections.revoked, 0);

    const revoked = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/revoke-connections',
        { method: 'POST', headers: { Authorization: 'Bearer admin-secret' } },
    ), env);
    const revokedPayload = await revoked.json();

    assert.equal(revoked.status, 200);
    assert.equal(revokedPayload.revoked_connections, 1);

    const blocked = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/analytics',
        { headers: { Authorization: `Bearer ${oauth.broker_connection}` } },
    ), env);

    assert.equal(blocked.status, 401);
    assert.equal((await blocked.json()).error, 'broker_connection_not_authorized');
});

test('Analytics refreshes expired TikTok access tokens before importing', async (t) => {
    const env = testEnv();
    const fetchMock = installTikTokFetchMock(t, {
        refreshedAccessToken: 'access-token-2',
    });
    const oauth = await completeOAuth(env);
    const account = env.DB.accounts.get('open_123');
    const originalCiphertext = account.access_token_ciphertext;

    account.access_token_expires_at = new Date(Date.now() - 60_000).toISOString();

    const response = await worker.fetch(new Request(
        'https://broker.example/api/tiktok/accounts/open_123/analytics',
        { headers: { Authorization: `Bearer ${oauth.broker_connection}` } },
    ), env);

    assert.equal(response.status, 200);
    assert.notEqual(account.access_token_ciphertext, originalCiphertext);
    assert.equal(fetchMock.tokenGrantTypes.join(','), 'authorization_code,refresh_token');
    assert.equal(fetchMock.accessTokensUsed.at(-1), 'Bearer access-token-2');
});

async function completeOAuth(env) {
    const start = await worker.fetch(
        new Request('https://broker.example/api/tiktok/oauth/start'),
        env,
    );
    const redirect = start.headers.get('location');
    const state = new URL(redirect).searchParams.get('state');

    assert.equal(start.status, 302);
    assert.ok(state);

    const callback = await worker.fetch(
        new Request(`https://broker.example/api/tiktok/oauth/callback?code=abc123&state=${state}&format=json`),
        env,
    );

    assert.equal(callback.status, 200);

    return callback.json();
}

function installTikTokFetchMock(t, options = {}) {
    const originalFetch = global.fetch;
    const calls = [];
    const tokenScope = options.tokenScope || 'user.info.basic,user.info.stats,video.list';
    const tokenGrantTypes = [];
    const accessTokensUsed = [];

    global.fetch = async (url, request = {}) => {
        const requestUrl = String(url);

        calls.push({ url: requestUrl, request });

        if (requestUrl.includes('/oauth/token/')) {
            const grantType = request.body?.get?.('grant_type') || '';

            tokenGrantTypes.push(grantType);

            return jsonResponse({
                data: {
                    access_token: grantType === 'refresh_token'
                        ? options.refreshedAccessToken || 'access-token-1'
                        : 'access-token-1',
                    refresh_token: grantType === 'refresh_token'
                        ? options.refreshedRefreshToken || 'refresh-token-2'
                        : 'refresh-token-1',
                    expires_in: 3600,
                    refresh_expires_in: 86400,
                    open_id: 'open_123',
                    scope: tokenScope,
                    token_type: 'Bearer',
                },
            });
        }

        if (requestUrl.includes('/user/info/')) {
            accessTokensUsed.push(request.headers?.Authorization || request.headers?.authorization || '');

            return jsonResponse({
                data: {
                    user: {
                        open_id: 'open_123',
                        union_id: 'union_123',
                        display_name: 'Dust Wave TikTok',
                        username: 'dustwave',
                        avatar_url: 'https://cdn.example/avatar.png',
                        profile_deep_link: 'https://www.tiktok.com/@dustwave',
                        follower_count: 4312,
                        following_count: 120,
                        likes_count: 9910,
                        video_count: 42,
                    },
                },
                error: { code: 'ok' },
            });
        }

        if (requestUrl.includes('/video/list/')) {
            accessTokensUsed.push(request.headers?.Authorization || request.headers?.authorization || '');

            return jsonResponse({
                data: {
                    videos: [
                        {
                            id: 'video_1',
                            create_time: 1717200000,
                            video_description: 'Launch clip',
                            share_url: 'https://www.tiktok.com/@dustwave/video/1',
                            view_count: 1200,
                            like_count: 88,
                            comment_count: 7,
                            share_count: 4,
                        },
                        {
                            id: 'video_2',
                            create_time: 1717286400,
                            video_description: 'Studio clip',
                            share_url: 'https://www.tiktok.com/@dustwave/video/2',
                            view_count: 900,
                            like_count: 61,
                            comment_count: 5,
                            share_count: 3,
                        },
                    ],
                    cursor: 0,
                    has_more: false,
                },
                error: { code: 'ok' },
            });
        }

        throw new Error(`Unexpected upstream request: ${requestUrl}`);
    };

    t.after(() => {
        global.fetch = originalFetch;
    });

    return { calls, tokenGrantTypes, accessTokensUsed };
}

function jsonResponse(body, status = 200) {
    return new Response(JSON.stringify(body), {
        status,
        headers: { 'Content-Type': 'application/json' },
    });
}

function testEnv(options = {}) {
    return {
        DB: new MemoryD1(),
        TIKTOK_CLIENT_KEY: 'client-key',
        TIKTOK_CLIENT_SECRET: 'client-secret',
        TOKEN_ENCRYPTION_KEY: tokenEncryptionKey,
        TIKTOK_SCOPES: options.scopes || 'user.info.basic,user.info.stats,video.list',
        BROKER_ADMIN_TOKEN: options.adminToken || 'admin-secret',
    };
}

class MemoryD1 {
    constructor() {
        this.oauthStates = new Map();
        this.accounts = new Map();
        this.connections = new Map();
    }

    prepare(sql) {
        return new MemoryStatement(this, sql);
    }

    async batch(statements) {
        for (const statement of statements) {
            await statement.run();
        }

        return statements.map(() => ({ success: true }));
    }
}

class MemoryStatement {
    constructor(db, sql) {
        this.db = db;
        this.sql = sql.replace(/\s+/g, ' ').trim();
        this.params = [];
    }

    bind(...params) {
        this.params = params;

        return this;
    }

    async first() {
        if (this.sql.startsWith('SELECT state, redirect_uri, scopes, expires_at, used_at FROM oauth_states')) {
            return this.db.oauthStates.get(this.params[0]) || null;
        }

        if (this.sql.startsWith('SELECT credential_hash, open_id, revoked_at FROM broker_connections')) {
            const [credentialHash, openId] = this.params;
            const connection = this.db.connections.get(credentialHash);

            return connection?.open_id === openId ? connection : null;
        }

        if (this.sql.startsWith('SELECT open_id, scopes, access_token_ciphertext')) {
            return this.db.accounts.get(this.params[0]) || null;
        }

        if (this.sql.startsWith('SELECT open_id, display_name, username, scopes')) {
            return this.db.accounts.get(this.params[0]) || null;
        }

        if (this.sql.startsWith('SELECT COUNT(*) AS total_connections')) {
            const [openId] = this.params;
            const connections = [...this.db.connections.values()]
                .filter((connection) => connection.open_id === openId);
            const revoked = connections.filter((connection) => connection.revoked_at).length;

            return {
                total_connections: connections.length,
                active_connections: connections.length - revoked,
                revoked_connections: revoked,
            };
        }

        if (this.sql.startsWith('SELECT open_id FROM tiktok_accounts')) {
            const account = this.db.accounts.get(this.params[0]);

            return account ? { open_id: account.open_id } : null;
        }

        throw new Error(`Unhandled first SQL: ${this.sql}`);
    }

    async run() {
        if (this.sql.startsWith('DELETE FROM oauth_states WHERE expires_at')) {
            const [now] = this.params;

            for (const [state, row] of this.db.oauthStates) {
                if (row.expires_at < now) {
                    this.db.oauthStates.delete(state);
                }
            }

            return { success: true };
        }

        if (this.sql.startsWith('INSERT INTO oauth_states')) {
            const [state, redirectUri, scopes, createdAt, expiresAt] = this.params;

            this.db.oauthStates.set(state, {
                state,
                redirect_uri: redirectUri,
                scopes,
                created_at: createdAt,
                expires_at: expiresAt,
                used_at: null,
            });

            return { success: true };
        }

        if (this.sql.startsWith('UPDATE oauth_states SET used_at')) {
            const [usedAt, state] = this.params;
            const row = this.db.oauthStates.get(state);

            if (row) {
                row.used_at = usedAt;
            }

            return { success: true };
        }

        if (this.sql.startsWith('INSERT INTO tiktok_accounts')) {
            const [
                openId,
                unionId,
                displayName,
                username,
                avatarUrl,
                profileDeepLink,
                scopes,
                accessTokenCiphertext,
                refreshTokenCiphertext,
                accessTokenExpiresAt,
                refreshTokenExpiresAt,
                tokenType,
                createdAt,
                updatedAt,
            ] = this.params;
            const existing = this.db.accounts.get(openId);

            this.db.accounts.set(openId, {
                open_id: openId,
                union_id: unionId,
                display_name: displayName,
                username,
                avatar_url: avatarUrl,
                profile_deep_link: profileDeepLink,
                scopes,
                access_token_ciphertext: accessTokenCiphertext,
                refresh_token_ciphertext: refreshTokenCiphertext,
                access_token_expires_at: accessTokenExpiresAt,
                refresh_token_expires_at: refreshTokenExpiresAt,
                token_type: tokenType,
                created_at: existing?.created_at || createdAt,
                updated_at: updatedAt,
            });

            return { success: true };
        }

        if (this.sql.startsWith('INSERT INTO broker_connections')) {
            const [credentialHash, openId, createdAt] = this.params;

            this.db.connections.set(credentialHash, {
                credential_hash: credentialHash,
                open_id: openId,
                created_at: createdAt,
                last_used_at: null,
                revoked_at: null,
            });

            return { success: true };
        }

        if (this.sql.startsWith('UPDATE broker_connections SET last_used_at')) {
            const [lastUsedAt, credentialHash] = this.params;
            const connection = this.db.connections.get(credentialHash);

            if (connection) {
                connection.last_used_at = lastUsedAt;
            }

            return { success: true };
        }

        if (this.sql.startsWith('UPDATE broker_connections SET revoked_at')) {
            const [revokedAt, openId] = this.params;
            let changes = 0;

            for (const connection of this.db.connections.values()) {
                if (connection.open_id === openId && !connection.revoked_at) {
                    connection.revoked_at = revokedAt;
                    changes += 1;
                }
            }

            return { success: true, meta: { changes } };
        }

        if (this.sql.startsWith('UPDATE tiktok_accounts SET display_name')) {
            const [displayName, username, avatarUrl, profileDeepLink, lastImportedAt, openId] = this.params;
            const account = this.db.accounts.get(openId);

            if (account) {
                account.display_name = displayName;
                account.username = username;
                account.avatar_url = avatarUrl;
                account.profile_deep_link = profileDeepLink;
                account.last_imported_at = lastImportedAt;
                account.updated_at = lastImportedAt;
            }

            return { success: true };
        }

        if (this.sql.startsWith('UPDATE tiktok_accounts SET access_token_ciphertext')) {
            const [
                accessTokenCiphertext,
                refreshTokenCiphertext,
                accessTokenExpiresAt,
                refreshTokenExpiresAt,
                updatedAt,
                openId,
            ] = this.params;
            const account = this.db.accounts.get(openId);

            if (account) {
                account.access_token_ciphertext = accessTokenCiphertext;
                account.refresh_token_ciphertext = refreshTokenCiphertext;
                account.access_token_expires_at = accessTokenExpiresAt;
                account.refresh_token_expires_at = refreshTokenExpiresAt;
                account.updated_at = updatedAt;
            }

            return { success: true };
        }

        throw new Error(`Unhandled run SQL: ${this.sql}`);
    }
}
