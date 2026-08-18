const DEFAULT_SCOPES = ["user.info.basic", "user.info.stats", "video.list"];
const STATE_TTL_SECONDS = 15 * 60;
const TOKEN_REFRESH_SKEW_SECONDS = 5 * 60;
const TIKTOK_AUTH_URL = "https://www.tiktok.com/v2/auth/authorize/";
const TIKTOK_TOKEN_URL = "https://open.tiktokapis.com/v2/oauth/token/";
const TIKTOK_USER_INFO_URL = "https://open.tiktokapis.com/v2/user/info/";
const TIKTOK_VIDEO_LIST_URL = "https://open.tiktokapis.com/v2/video/list/";

export default {
    async fetch(request, env) {
        try {
            if (request.method === "OPTIONS") {
                return new Response(null, { status: 204, headers: corsHeaders() });
            }

            const url = new URL(request.url);

            if (url.pathname === "/api/health" && request.method === "GET") {
                return jsonResponse({ ok: true, service: "dustwave-tiktok-broker" });
            }

            if (url.pathname === "/api/tiktok/oauth/start" && request.method === "GET") {
                return await startOAuth(request, env, url);
            }

            if (url.pathname === "/api/tiktok/oauth/callback" && request.method === "GET") {
                return await completeOAuth(env, url);
            }

            const analyticsMatch = url.pathname.match(/^\/api\/tiktok\/accounts\/([^/]+)\/analytics$/);

            if (analyticsMatch && request.method === "GET") {
                return await accountAnalytics(request, env, decodeURIComponent(analyticsMatch[1]));
            }

            const accountStatusMatch = url.pathname.match(/^\/api\/tiktok\/accounts\/([^/]+)\/status$/);

            if (accountStatusMatch && request.method === "GET") {
                return await accountStatus(request, env, decodeURIComponent(accountStatusMatch[1]));
            }

            const accountRevocationMatch = url.pathname.match(/^\/api\/tiktok\/accounts\/([^/]+)\/revoke-connections$/);

            if (accountRevocationMatch && request.method === "POST") {
                return await revokeAccountConnections(request, env, decodeURIComponent(accountRevocationMatch[1]));
            }

            return jsonResponse({ error: "not_found" }, 404);
        } catch (error) {
            if (!(error instanceof BrokerError) || error.status >= 500) {
                console.error(error);
            }
            const status = error instanceof BrokerError ? error.status : 500;
            const message = error instanceof BrokerError ? error.message : "internal_server_error";

            return jsonResponse({ error: message }, status);
        }
    },
};

async function startOAuth(request, env, url) {
    requireEnv(env, ["DB", "TIKTOK_CLIENT_KEY"]);
    await purgeExpiredStates(env);

    const callbackUrl = brokerCallbackUrl(request, env);
    const scopes = requestedScopes(url.searchParams.get("scopes") || env.TIKTOK_SCOPES);
    const state = randomToken(24);
    const now = new Date();
    const expiresAt = new Date(now.getTime() + STATE_TTL_SECONDS * 1000);

    await env.DB.prepare(
        `INSERT INTO oauth_states (state, redirect_uri, scopes, created_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5)`
    )
        .bind(state, callbackUrl, scopes.join(","), now.toISOString(), expiresAt.toISOString())
        .run();

    const authUrl = new URL(TIKTOK_AUTH_URL);
    authUrl.searchParams.set("client_key", env.TIKTOK_CLIENT_KEY);
    authUrl.searchParams.set("scope", scopes.join(","));
    authUrl.searchParams.set("response_type", "code");
    authUrl.searchParams.set("redirect_uri", callbackUrl);
    authUrl.searchParams.set("state", state);

    return Response.redirect(authUrl.toString(), 302);
}

async function completeOAuth(env, url) {
    requireEnv(env, ["DB", "TIKTOK_CLIENT_KEY", "TIKTOK_CLIENT_SECRET", "TOKEN_ENCRYPTION_KEY"]);

    const code = requiredParam(url, "code");
    const state = requiredParam(url, "state");
    const row = await env.DB.prepare(
        `SELECT state, redirect_uri, scopes, expires_at, used_at
         FROM oauth_states
         WHERE state = ?1`
    )
        .bind(state)
        .first();

    if (!row) {
        throw new BrokerError("oauth_state_not_found", 400);
    }
    if (row.used_at) {
        throw new BrokerError("oauth_state_already_used", 400);
    }
    if (Date.parse(row.expires_at) < Date.now()) {
        throw new BrokerError("oauth_state_expired", 400);
    }

    await env.DB.prepare("UPDATE oauth_states SET used_at = ?1 WHERE state = ?2")
        .bind(new Date().toISOString(), state)
        .run();

    const tokenSet = await exchangeAuthorizationCode(env, code, row.redirect_uri);
    const scopes = requestedScopes(tokenSet.scope || row.scopes);
    const user = await fetchTikTokUser(tokenSet.access_token, scopes);
    const openId = tokenSet.open_id || user.open_id;

    if (!openId) {
        throw new BrokerError("tiktok_open_id_missing", 502);
    }

    const storedUser = { ...user, open_id: openId };
    const connectionCredential = `dw_tiktok_${randomToken(32)}`;
    const credentialHash = await sha256Hex(connectionCredential);
    const now = new Date();
    const accessTokenExpiresAt = dateAfterSeconds(now, tokenSet.expires_in || 86400);
    const refreshTokenExpiresAt = tokenSet.refresh_expires_in
        ? dateAfterSeconds(now, tokenSet.refresh_expires_in)
        : null;

    await storeTikTokAccount(env, {
        openId,
        user: storedUser,
        scopes,
        accessTokenCiphertext: await encryptSecret(env, tokenSet.access_token),
        refreshTokenCiphertext: tokenSet.refresh_token
            ? await encryptSecret(env, tokenSet.refresh_token)
            : null,
        accessTokenExpiresAt,
        refreshTokenExpiresAt,
        tokenType: tokenSet.token_type || "Bearer",
        now,
    });

    await env.DB.prepare(
        `INSERT INTO broker_connections (credential_hash, open_id, created_at)
         VALUES (?1, ?2, ?3)`
    )
        .bind(credentialHash, openId, now.toISOString())
        .run();

    const payload = {
        provider: "tiktok",
        provider_id: openId,
        name: storedUser.display_name || "TikTok",
        username: storedUser.username || null,
        scopes,
        broker_connection: connectionCredential,
    };

    if (url.searchParams.get("format") === "json") {
        return jsonResponse(payload);
    }

    return htmlResponse(oauthCompleteHtml(payload));
}

async function accountAnalytics(request, env, openId) {
    requireEnv(env, ["DB", "TIKTOK_CLIENT_KEY", "TIKTOK_CLIENT_SECRET", "TOKEN_ENCRYPTION_KEY"]);

    const credential = bearerToken(request);
    const credentialHash = await sha256Hex(credential);
    const connection = await env.DB.prepare(
        `SELECT credential_hash, open_id, revoked_at
         FROM broker_connections
         WHERE credential_hash = ?1 AND open_id = ?2`
    )
        .bind(credentialHash, openId)
        .first();

    if (!connection || connection.revoked_at) {
        throw new BrokerError("broker_connection_not_authorized", 401);
    }

    const account = await env.DB.prepare(
        `SELECT open_id, scopes, access_token_ciphertext, refresh_token_ciphertext,
                access_token_expires_at, refresh_token_expires_at
         FROM tiktok_accounts
         WHERE open_id = ?1`
    )
        .bind(openId)
        .first();

    if (!account) {
        throw new BrokerError("tiktok_account_not_found", 404);
    }

    const tokenSet = await freshAccessToken(env, account);
    const scopes = requestedScopes(account.scopes);
    const user = await fetchTikTokUser(tokenSet.accessToken, scopes);
    const videos = await fetchTikTokVideos(tokenSet.accessToken, scopes, env);
    const now = new Date().toISOString();

    await env.DB.batch([
        env.DB.prepare(
            `UPDATE broker_connections
             SET last_used_at = ?1
             WHERE credential_hash = ?2`
        ).bind(now, credentialHash),
        env.DB.prepare(
            `UPDATE tiktok_accounts
             SET display_name = ?1,
                 username = ?2,
                 avatar_url = ?3,
                 profile_deep_link = ?4,
                 last_imported_at = ?5,
                 updated_at = ?5
             WHERE open_id = ?6`
        ).bind(
            user.display_name || null,
            user.username || null,
            user.avatar_url || user.avatar_url_100 || user.avatar_large_url || null,
            user.profile_deep_link || null,
            now,
            openId
        ),
    ]);

    return jsonResponse({
        provider: "tiktok",
        provider_id: openId,
        imported_at: now,
        user: { ...user, open_id: openId },
        videos,
    });
}

async function accountStatus(request, env, openId) {
    await requireAdminToken(request, env);
    requireEnv(env, ["DB"]);

    const account = await env.DB.prepare(
        `SELECT open_id, display_name, username, scopes, last_imported_at, updated_at
         FROM tiktok_accounts
         WHERE open_id = ?1`
    )
        .bind(openId)
        .first();

    if (!account) {
        throw new BrokerError("tiktok_account_not_found", 404);
    }

    const connections = await env.DB.prepare(
        `SELECT
            COUNT(*) AS total_connections,
            SUM(CASE WHEN revoked_at IS NULL THEN 1 ELSE 0 END) AS active_connections,
            SUM(CASE WHEN revoked_at IS NOT NULL THEN 1 ELSE 0 END) AS revoked_connections
         FROM broker_connections
         WHERE open_id = ?1`
    )
        .bind(openId)
        .first();

    return jsonResponse({
        provider: "tiktok",
        provider_id: openId,
        display_name: account.display_name || null,
        username: account.username || null,
        scopes: requestedScopes(account.scopes),
        last_imported_at: account.last_imported_at || null,
        updated_at: account.updated_at || null,
        connections: {
            total: Number(connections?.total_connections || 0),
            active: Number(connections?.active_connections || 0),
            revoked: Number(connections?.revoked_connections || 0),
        },
    });
}

async function revokeAccountConnections(request, env, openId) {
    await requireAdminToken(request, env);
    requireEnv(env, ["DB"]);

    const account = await env.DB.prepare(
        `SELECT open_id
         FROM tiktok_accounts
         WHERE open_id = ?1`
    )
        .bind(openId)
        .first();

    if (!account) {
        throw new BrokerError("tiktok_account_not_found", 404);
    }

    const now = new Date().toISOString();
    const result = await env.DB.prepare(
        `UPDATE broker_connections
         SET revoked_at = ?1
         WHERE open_id = ?2 AND revoked_at IS NULL`
    )
        .bind(now, openId)
        .run();

    return jsonResponse({
        provider: "tiktok",
        provider_id: openId,
        revoked_at: now,
        revoked_connections: Number(result?.meta?.changes ?? 0),
    });
}

async function exchangeAuthorizationCode(env, code, redirectUri) {
    const body = new URLSearchParams();
    body.set("client_key", env.TIKTOK_CLIENT_KEY);
    body.set("client_secret", env.TIKTOK_CLIENT_SECRET);
    body.set("code", code);
    body.set("grant_type", "authorization_code");
    body.set("redirect_uri", redirectUri);

    return tiktokTokenRequest(body);
}

async function refreshAccessToken(env, refreshToken) {
    const body = new URLSearchParams();
    body.set("client_key", env.TIKTOK_CLIENT_KEY);
    body.set("client_secret", env.TIKTOK_CLIENT_SECRET);
    body.set("grant_type", "refresh_token");
    body.set("refresh_token", refreshToken);

    return tiktokTokenRequest(body);
}

async function tiktokTokenRequest(body) {
    const response = await fetch(TIKTOK_TOKEN_URL, {
        method: "POST",
        headers: { "Content-Type": "application/x-www-form-urlencoded" },
        body,
    });
    const payload = await readJson(response);
    const data = payload.data || payload;

    if (!response.ok || payload.error || data.error_code) {
        throw new BrokerError(tiktokErrorMessage(payload, "tiktok_token_request_failed"), 502);
    }
    if (!data.access_token) {
        throw new BrokerError("tiktok_access_token_missing", 502);
    }

    return data;
}

async function freshAccessToken(env, account) {
    const expiresAt = Date.parse(account.access_token_expires_at);
    const shouldRefresh = Number.isFinite(expiresAt)
        && expiresAt - TOKEN_REFRESH_SKEW_SECONDS * 1000 <= Date.now();

    if (!shouldRefresh) {
        return { accessToken: await decryptSecret(env, account.access_token_ciphertext) };
    }

    if (!account.refresh_token_ciphertext) {
        throw new BrokerError("tiktok_refresh_token_missing", 401);
    }

    const refreshToken = await decryptSecret(env, account.refresh_token_ciphertext);
    const tokenSet = await refreshAccessToken(env, refreshToken);
    const now = new Date();
    const accessTokenExpiresAt = dateAfterSeconds(now, tokenSet.expires_in || 86400);
    const refreshTokenExpiresAt = tokenSet.refresh_expires_in
        ? dateAfterSeconds(now, tokenSet.refresh_expires_in)
        : account.refresh_token_expires_at;
    const nextRefreshToken = tokenSet.refresh_token || refreshToken;

    await env.DB.prepare(
        `UPDATE tiktok_accounts
         SET access_token_ciphertext = ?1,
             refresh_token_ciphertext = ?2,
             access_token_expires_at = ?3,
             refresh_token_expires_at = ?4,
             updated_at = ?5
         WHERE open_id = ?6`
    )
        .bind(
            await encryptSecret(env, tokenSet.access_token),
            await encryptSecret(env, nextRefreshToken),
            accessTokenExpiresAt,
            refreshTokenExpiresAt,
            now.toISOString(),
            account.open_id
        )
        .run();

    return { accessToken: tokenSet.access_token };
}

async function fetchTikTokUser(accessToken, scopes) {
    const fields = [
        "open_id",
        "union_id",
        "avatar_url",
        "avatar_url_100",
        "avatar_large_url",
        "display_name",
    ];

    if (scopes.includes("user.info.profile")) {
        fields.push("bio_description", "profile_deep_link", "is_verified", "username");
    }
    if (scopes.includes("user.info.stats")) {
        fields.push("follower_count", "following_count", "likes_count", "video_count");
    }

    const url = new URL(TIKTOK_USER_INFO_URL);
    url.searchParams.set("fields", fields.join(","));

    const response = await fetch(url.toString(), {
        headers: { Authorization: `Bearer ${accessToken}` },
    });
    const payload = await readJson(response);

    if (!response.ok || payload.error?.code && payload.error.code !== "ok") {
        throw new BrokerError(tiktokErrorMessage(payload, "tiktok_user_info_failed"), 502);
    }

    return payload.data?.user || {};
}

async function fetchTikTokVideos(accessToken, scopes, env) {
    if (!scopes.includes("video.list")) {
        return [];
    }

    const fields = [
        "id",
        "create_time",
        "cover_image_url",
        "share_url",
        "video_description",
        "duration",
        "height",
        "width",
        "title",
        "embed_link",
        "like_count",
        "comment_count",
        "share_count",
        "view_count",
    ];
    const url = new URL(TIKTOK_VIDEO_LIST_URL);
    url.searchParams.set("fields", fields.join(","));

    const videos = [];
    const pageLimit = Math.max(1, Math.min(Number(env.TIKTOK_VIDEO_PAGE_LIMIT || 5), 25));
    let cursor = null;

    for (let page = 0; page < pageLimit; page += 1) {
        const body = { max_count: 20 };

        if (cursor !== null && cursor !== undefined) {
            body.cursor = cursor;
        }

        const response = await fetch(url.toString(), {
            method: "POST",
            headers: {
                Authorization: `Bearer ${accessToken}`,
                "Content-Type": "application/json",
            },
            body: JSON.stringify(body),
        });
        const payload = await readJson(response);

        if (!response.ok || payload.error?.code && payload.error.code !== "ok") {
            throw new BrokerError(tiktokErrorMessage(payload, "tiktok_video_list_failed"), 502);
        }

        const data = payload.data || {};
        videos.push(...(Array.isArray(data.videos) ? data.videos : []));

        if (!data.has_more) {
            break;
        }

        cursor = data.cursor;
    }

    return videos;
}

async function storeTikTokAccount(env, input) {
    await env.DB.prepare(
        `INSERT INTO tiktok_accounts (
            open_id, union_id, display_name, username, avatar_url, profile_deep_link, scopes,
            access_token_ciphertext, refresh_token_ciphertext, access_token_expires_at,
            refresh_token_expires_at, token_type, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
         ON CONFLICT(open_id) DO UPDATE SET
            union_id = excluded.union_id,
            display_name = excluded.display_name,
            username = excluded.username,
            avatar_url = excluded.avatar_url,
            profile_deep_link = excluded.profile_deep_link,
            scopes = excluded.scopes,
            access_token_ciphertext = excluded.access_token_ciphertext,
            refresh_token_ciphertext = excluded.refresh_token_ciphertext,
            access_token_expires_at = excluded.access_token_expires_at,
            refresh_token_expires_at = excluded.refresh_token_expires_at,
            token_type = excluded.token_type,
            updated_at = excluded.updated_at`
    )
        .bind(
            input.openId,
            input.user.union_id || null,
            input.user.display_name || null,
            input.user.username || null,
            input.user.avatar_url || input.user.avatar_url_100 || input.user.avatar_large_url || null,
            input.user.profile_deep_link || null,
            input.scopes.join(","),
            input.accessTokenCiphertext,
            input.refreshTokenCiphertext,
            input.accessTokenExpiresAt,
            input.refreshTokenExpiresAt,
            input.tokenType,
            input.now.toISOString(),
            input.now.toISOString()
        )
        .run();
}

async function purgeExpiredStates(env) {
    await env.DB.prepare("DELETE FROM oauth_states WHERE expires_at < ?1")
        .bind(new Date().toISOString())
        .run();
}

function brokerCallbackUrl(request, env) {
    const baseUrl = env.PUBLIC_BROKER_BASE_URL || new URL(request.url).origin;
    const url = new URL(baseUrl);

    url.pathname = "/api/tiktok/oauth/callback";
    url.search = "";
    url.hash = "";

    return url.toString();
}

function requestedScopes(value) {
    const scopes = String(value || "")
        .split(/[,\s]+/)
        .map((scope) => scope.trim())
        .filter(Boolean);

    return scopes.length > 0 ? [...new Set(scopes)] : DEFAULT_SCOPES;
}

function bearerToken(request) {
    const header = request.headers.get("Authorization") || "";
    const match = header.match(/^Bearer\s+(.+)$/i);

    if (!match || !match[1].trim()) {
        throw new BrokerError("missing_bearer_token", 401);
    }

    return match[1].trim();
}

async function requireAdminToken(request, env) {
    requireEnv(env, ["BROKER_ADMIN_TOKEN"]);

    const token = bearerToken(request);
    const provided = await sha256Hex(token);
    const expected = await sha256Hex(String(env.BROKER_ADMIN_TOKEN));

    if (provided !== expected) {
        throw new BrokerError("admin_not_authorized", 401);
    }
}

function requiredParam(url, key) {
    const value = url.searchParams.get(key);

    if (!value) {
        throw new BrokerError(`missing_${key}`, 400);
    }

    return value;
}

function requireEnv(env, keys) {
    for (const key of keys) {
        if (!env[key]) {
            throw new BrokerError(`missing_env_${key.toLowerCase()}`, 500);
        }
    }
}

async function readJson(response) {
    const text = await response.text();

    if (!text) {
        return {};
    }

    try {
        return JSON.parse(text);
    } catch {
        throw new BrokerError("upstream_returned_invalid_json", 502);
    }
}

function tiktokErrorMessage(payload, fallback) {
    return payload.error?.message
        || payload.error_description
        || payload.message
        || payload.description
        || fallback;
}

async function encryptSecret(env, value) {
    const iv = crypto.getRandomValues(new Uint8Array(12));
    const key = await encryptionKey(env);
    const encoded = new TextEncoder().encode(value);
    const ciphertext = await crypto.subtle.encrypt({ name: "AES-GCM", iv }, key, encoded);

    return `v1.${base64Encode(iv)}.${base64Encode(new Uint8Array(ciphertext))}`;
}

async function decryptSecret(env, stored) {
    const parts = String(stored || "").split(".");

    if (parts.length !== 3 || parts[0] !== "v1") {
        throw new BrokerError("invalid_encrypted_secret", 500);
    }

    const key = await encryptionKey(env);
    const iv = base64Decode(parts[1]);
    const ciphertext = base64Decode(parts[2]);
    const plaintext = await crypto.subtle.decrypt({ name: "AES-GCM", iv }, key, ciphertext);

    return new TextDecoder().decode(plaintext);
}

async function encryptionKey(env) {
    const raw = decodeKeyMaterial(env.TOKEN_ENCRYPTION_KEY);

    if (raw.byteLength !== 32) {
        throw new BrokerError("token_encryption_key_must_be_32_bytes", 500);
    }

    return crypto.subtle.importKey("raw", raw, { name: "AES-GCM" }, false, ["encrypt", "decrypt"]);
}

function decodeKeyMaterial(value) {
    const trimmed = String(value || "").trim();

    if (/^[a-f0-9]{64}$/i.test(trimmed)) {
        const bytes = new Uint8Array(32);

        for (let i = 0; i < bytes.length; i += 1) {
            bytes[i] = Number.parseInt(trimmed.slice(i * 2, i * 2 + 2), 16);
        }

        return bytes;
    }

    return base64Decode(trimmed);
}

async function sha256Hex(value) {
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));

    return [...new Uint8Array(digest)]
        .map((byte) => byte.toString(16).padStart(2, "0"))
        .join("");
}

function randomToken(bytes = 24) {
    const buffer = crypto.getRandomValues(new Uint8Array(bytes));

    return base64UrlEncode(buffer);
}

function dateAfterSeconds(date, seconds) {
    return new Date(date.getTime() + seconds * 1000).toISOString();
}

function jsonResponse(body, status = 200) {
    return new Response(JSON.stringify(body, null, 2), {
        status,
        headers: {
            "Content-Type": "application/json; charset=utf-8",
            ...corsHeaders(),
        },
    });
}

function htmlResponse(body, status = 200) {
    return new Response(body, {
        status,
        headers: {
            "Content-Type": "text/html; charset=utf-8",
            "Cache-Control": "no-store",
        },
    });
}

function corsHeaders() {
    return {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET,POST,OPTIONS",
        "Access-Control-Allow-Headers": "Authorization,Content-Type",
    };
}

function oauthCompleteHtml(payload) {
    const json = JSON.stringify(payload, null, 2);

    return `<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>TikTok Connected</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; margin: 2rem; max-width: 760px; line-height: 1.5; }
        code, pre { background: #f3f4f6; border-radius: 6px; }
        code { padding: 0.1rem 0.25rem; }
        pre { overflow: auto; padding: 1rem; }
    </style>
</head>
<body>
    <h1>TikTok Connected</h1>
    <p>Copy these values into Dust Wave Social's TikTok account form. The broker connection credential is shown once.</p>
    <p><strong>TikTok user ID:</strong> <code>${escapeHtml(payload.provider_id)}</code></p>
    <p><strong>Display name:</strong> <code>${escapeHtml(payload.name)}</code></p>
    <p><strong>Username:</strong> <code>${escapeHtml(payload.username || "")}</code></p>
    <p><strong>Granted scopes:</strong> <code>${escapeHtml(payload.scopes.join(","))}</code></p>
    <p><strong>Broker connection credential:</strong> <code>${escapeHtml(payload.broker_connection)}</code></p>
    <pre>${escapeHtml(json)}</pre>
</body>
</html>`;
}

function escapeHtml(value) {
    return String(value)
        .replaceAll("&", "&amp;")
        .replaceAll("<", "&lt;")
        .replaceAll(">", "&gt;")
        .replaceAll('"', "&quot;")
        .replaceAll("'", "&#039;");
}

function base64Encode(bytes) {
    let binary = "";

    for (const byte of bytes) {
        binary += String.fromCharCode(byte);
    }

    return btoa(binary);
}

function base64Decode(value) {
    const binary = atob(value);
    const bytes = new Uint8Array(binary.length);

    for (let i = 0; i < binary.length; i += 1) {
        bytes[i] = binary.charCodeAt(i);
    }

    return bytes;
}

function base64UrlEncode(bytes) {
    return base64Encode(bytes)
        .replaceAll("+", "-")
        .replaceAll("/", "_")
        .replaceAll("=", "");
}

class BrokerError extends Error {
    constructor(message, status = 400) {
        super(message);
        this.status = status;
    }
}
