const DEFAULT_MAX_OBJECT_BYTES = 25 * 1024 * 1024;
const DEFAULT_TTL_SECONDS = 24 * 60 * 60;
const MAX_TTL_SECONDS = 7 * 24 * 60 * 60;
const ALLOWED_MIME_TYPES = new Set([
    "image/jpeg",
    "image/png",
    "image/webp",
    "video/mp4",
    "video/quicktime",
]);

export default {
    async fetch(request, env) {
        try {
            if (request.method === "OPTIONS") {
                return new Response(null, { status: 204, headers: corsHeaders() });
            }

            const url = new URL(request.url);

            if (url.pathname === "/api/health" && request.method === "GET") {
                return jsonResponse({ ok: true, service: "dustwave-media-staging" });
            }

            if (url.pathname === "/api/media/stage" && request.method === "POST") {
                return await stageMedia(request, env);
            }

            const mediaMatch = url.pathname.match(/^\/media\/([A-Za-z0-9._/-]+)$/);

            if (mediaMatch && request.method === "GET") {
                return await serveMedia(request, env, mediaMatch[1]);
            }

            if (mediaMatch && request.method === "DELETE") {
                await requireToken(request, env);
                await env.MEDIA_BUCKET.delete(mediaMatch[1]);
                return jsonResponse({ ok: true, deleted: true });
            }

            if (url.pathname === "/api/media/cleanup" && request.method === "POST") {
                await requireToken(request, env);
                return jsonResponse(await cleanupExpired(env));
            }

            return jsonResponse({ error: "not_found" }, 404);
        } catch (error) {
            if (!(error instanceof StagingError) || error.status >= 500) {
                console.error(error);
            }

            return jsonResponse(
                { error: error instanceof StagingError ? error.message : "internal_server_error" },
                error instanceof StagingError ? error.status : 500,
            );
        }
    },

    async scheduled(controller, env, ctx) {
        ctx.waitUntil(
            cleanupExpired(env)
                .then((summary) => {
                    console.log("media staging cleanup complete", {
                        cron: controller.cron,
                        scheduledTime: controller.scheduledTime,
                        scanned: summary.scanned,
                        deleted: summary.deleted,
                    });
                })
                .catch((error) => {
                    console.error("media staging cleanup failed", error);
                    throw error;
                }),
        );
    },
};

async function stageMedia(request, env) {
    requireEnv(env, ["MEDIA_BUCKET", "MEDIA_STAGING_TOKEN", "PUBLIC_MEDIA_BASE_URL"]);
    await requireToken(request, env);

    const contentType = trimContentType(request.headers.get("content-type") || "");

    if (!ALLOWED_MIME_TYPES.has(contentType)) {
        throw new StagingError(`unsupported_media_type:${contentType || "missing"}`, 415);
    }

    const declaredLength = Number(request.headers.get("content-length") || 0);
    const maxObjectBytes = positiveInteger(env.MAX_OBJECT_BYTES, DEFAULT_MAX_OBJECT_BYTES);

    if (declaredLength > maxObjectBytes) {
        throw new StagingError("media_too_large", 413);
    }

    const bytes = await request.arrayBuffer();

    if (bytes.byteLength <= 0) {
        throw new StagingError("media_body_required", 400);
    }

    if (bytes.byteLength > maxObjectBytes) {
        throw new StagingError("media_too_large", 413);
    }

    const signatureMimeType = sniffMimeType(new Uint8Array(bytes));

    if (signatureMimeType && signatureMimeType !== contentType) {
        throw new StagingError("media_signature_mismatch", 415);
    }

    const ttlSeconds = boundedTtl(request.headers.get("x-dustwave-ttl-seconds"), env.DEFAULT_TTL_SECONDS);
    const now = new Date();
    const expiresAt = new Date(now.getTime() + ttlSeconds * 1000);
    const sourceMediaId = cleanMetadata(request.headers.get("x-dustwave-source-media-id"));
    const operation = cleanMetadata(request.headers.get("x-dustwave-operation")) || "instagram_publish";
    const key = objectKey(contentType);

    await env.MEDIA_BUCKET.put(key, bytes, {
        httpMetadata: {
            contentType,
            cacheControl: `public, max-age=${Math.min(ttlSeconds, 3600)}`,
        },
        customMetadata: {
            sourceMediaId,
            operation,
            createdAt: now.toISOString(),
            expiresAt: expiresAt.toISOString(),
        },
    });

    const publicUrl = `${String(env.PUBLIC_MEDIA_BASE_URL).replace(/\/+$/, "")}/media/${key}`;

    return jsonResponse({
        ok: true,
        key,
        url: publicUrl,
        content_type: contentType,
        bytes: bytes.byteLength,
        expires_at: expiresAt.toISOString(),
    });
}

async function serveMedia(request, env, key) {
    requireEnv(env, ["MEDIA_BUCKET"]);

    const object = await env.MEDIA_BUCKET.get(key, {
        onlyIf: request.headers,
        range: request.headers,
    });

    if (!object) {
        return new Response("Not found", { status: 404 });
    }

    const expiresAt = object.customMetadata?.expiresAt;

    if (expiresAt && Date.parse(expiresAt) < Date.now()) {
        await env.MEDIA_BUCKET.delete(key);
        return new Response("Expired", { status: 410 });
    }

    const headers = new Headers();
    object.writeHttpMetadata(headers);
    headers.set("etag", object.httpEtag);
    headers.set("access-control-allow-origin", "*");

    return new Response(object.body, { headers });
}

async function cleanupExpired(env) {
    requireEnv(env, ["MEDIA_BUCKET"]);

    let cursor;
    let scanned = 0;
    let deleted = 0;

    do {
        const list = await env.MEDIA_BUCKET.list({ cursor, limit: 1000 });
        cursor = list.truncated ? list.cursor : undefined;

        for (const object of list.objects) {
            scanned += 1;
            const expiresAt = object.customMetadata?.expiresAt;

            if (expiresAt && Date.parse(expiresAt) < Date.now()) {
                await env.MEDIA_BUCKET.delete(object.key);
                deleted += 1;
            }
        }
    } while (cursor);

    return { ok: true, scanned, deleted };
}

async function requireToken(request, env) {
    const expected = String(env.MEDIA_STAGING_TOKEN || "").trim();

    if (!expected) {
        throw new StagingError("media_staging_token_missing", 500);
    }

    const token = bearerToken(request);

    if (!token) {
        throw new StagingError("missing_bearer_token", 401);
    }

    if (await sha256Hex(token) !== await sha256Hex(expected)) {
        throw new StagingError("media_staging_not_authorized", 401);
    }
}

function bearerToken(request) {
    const header = request.headers.get("authorization") || "";
    const match = header.match(/^Bearer\s+(.+)$/i);

    return match?.[1]?.trim() || "";
}

function requireEnv(env, names) {
    for (const name of names) {
        if (!env[name]) {
            throw new StagingError(`missing_env:${name}`, 500);
        }
    }
}

function objectKey(contentType) {
    const extension = extensionForMimeType(contentType);
    const date = new Date().toISOString().slice(0, 10);

    return `${date}/${crypto.randomUUID()}.${extension}`;
}

function extensionForMimeType(contentType) {
    switch (contentType) {
        case "image/jpeg":
            return "jpg";
        case "image/png":
            return "png";
        case "image/webp":
            return "webp";
        case "video/mp4":
            return "mp4";
        case "video/quicktime":
            return "mov";
        default:
            return "bin";
    }
}

function sniffMimeType(bytes) {
    if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
        return "image/jpeg";
    }

    if (
        bytes.length >= 8 &&
        bytes[0] === 0x89 &&
        bytes[1] === 0x50 &&
        bytes[2] === 0x4e &&
        bytes[3] === 0x47 &&
        bytes[4] === 0x0d &&
        bytes[5] === 0x0a &&
        bytes[6] === 0x1a &&
        bytes[7] === 0x0a
    ) {
        return "image/png";
    }

    if (
        bytes.length >= 12 &&
        bytes[0] === 0x52 &&
        bytes[1] === 0x49 &&
        bytes[2] === 0x46 &&
        bytes[3] === 0x46 &&
        bytes[8] === 0x57 &&
        bytes[9] === 0x45 &&
        bytes[10] === 0x42 &&
        bytes[11] === 0x50
    ) {
        return "image/webp";
    }

    if (bytes.length >= 12 && bytes[4] === 0x66 && bytes[5] === 0x74 && bytes[6] === 0x79 && bytes[7] === 0x70) {
        return null;
    }

    return null;
}

function positiveInteger(value, fallback) {
    const number = Number.parseInt(String(value || ""), 10);

    return Number.isFinite(number) && number > 0 ? number : fallback;
}

function boundedTtl(headerValue, envValue) {
    const ttl = positiveInteger(headerValue, positiveInteger(envValue, DEFAULT_TTL_SECONDS));

    return Math.max(60, Math.min(ttl, MAX_TTL_SECONDS));
}

function cleanMetadata(value) {
    return String(value || "")
        .trim()
        .replace(/[^\w:.-]/g, "_")
        .slice(0, 200);
}

async function sha256Hex(value) {
    const bytes = new TextEncoder().encode(value);
    const digest = await crypto.subtle.digest("SHA-256", bytes);

    return [...new Uint8Array(digest)]
        .map((byte) => byte.toString(16).padStart(2, "0"))
        .join("");
}

function trimContentType(value) {
    return String(value || "")
        .split(";")[0]
        .trim()
        .toLowerCase();
}

function corsHeaders() {
    return {
        "access-control-allow-origin": "*",
        "access-control-allow-methods": "GET,POST,DELETE,OPTIONS",
        "access-control-allow-headers": "authorization,content-type,x-dustwave-operation,x-dustwave-source-media-id,x-dustwave-ttl-seconds",
    };
}

function jsonResponse(payload, status = 200) {
    return new Response(JSON.stringify(payload), {
        status,
        headers: {
            "content-type": "application/json; charset=utf-8",
            ...corsHeaders(),
        },
    });
}

class StagingError extends Error {
    constructor(message, status = 400) {
        super(message);
        this.status = status;
    }
}
