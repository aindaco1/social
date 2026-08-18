import assert from 'node:assert/strict';
import { test } from 'node:test';
import worker from '../src/index.js';

test('stages media into R2 and serves it through public media URL', async () => {
    const env = testEnv();
    const bytes = pngBytes();
    const response = await worker.fetch(new Request(
        'https://media.example/api/media/stage',
        {
            method: 'POST',
            headers: {
                Authorization: 'Bearer staging-secret',
                'content-type': 'image/png',
                'x-dustwave-source-media-id': '42',
                'x-dustwave-operation': 'instagram_publish',
                'x-dustwave-ttl-seconds': '600',
            },
            body: bytes,
        },
    ), env);
    const payload = await response.json();

    assert.equal(response.status, 200);
    assert.equal(payload.content_type, 'image/png');
    assert.equal(payload.bytes, bytes.length);
    assert.match(payload.key, /^\d{4}-\d{2}-\d{2}\/.+\.png$/);
    assert.equal(payload.url, `https://media.example/media/${payload.key}`);

    const object = env.MEDIA_BUCKET.objects.get(payload.key);

    assert.ok(object);
    assert.equal(object.customMetadata.sourceMediaId, '42');
    assert.equal(object.customMetadata.operation, 'instagram_publish');

    const publicResponse = await worker.fetch(
        new Request(`https://media.example/media/${payload.key}`),
        env,
    );

    assert.equal(publicResponse.status, 200);
    assert.equal(publicResponse.headers.get('content-type'), 'image/png');
    assert.deepEqual(new Uint8Array(await publicResponse.arrayBuffer()), bytes);
});

test('staging rejects missing auth and MIME signature mismatch', async () => {
    const env = testEnv();
    const missingAuth = await worker.fetch(new Request(
        'https://media.example/api/media/stage',
        {
            method: 'POST',
            headers: { 'content-type': 'image/png' },
            body: pngBytes(),
        },
    ), env);

    assert.equal(missingAuth.status, 401);
    assert.equal((await missingAuth.json()).error, 'missing_bearer_token');

    const mismatch = await worker.fetch(new Request(
        'https://media.example/api/media/stage',
        {
            method: 'POST',
            headers: {
                Authorization: 'Bearer staging-secret',
                'content-type': 'image/jpeg',
            },
            body: pngBytes(),
        },
    ), env);

    assert.equal(mismatch.status, 415);
    assert.equal((await mismatch.json()).error, 'media_signature_mismatch');
});

test('cleanup removes expired staged media', async () => {
    const env = testEnv();

    await env.MEDIA_BUCKET.put('old.png', pngBytes(), {
        httpMetadata: { contentType: 'image/png' },
        customMetadata: {
            expiresAt: new Date(Date.now() - 1_000).toISOString(),
        },
    });
    await env.MEDIA_BUCKET.put('new.png', pngBytes(), {
        httpMetadata: { contentType: 'image/png' },
        customMetadata: {
            expiresAt: new Date(Date.now() + 60_000).toISOString(),
        },
    });

    const response = await worker.fetch(new Request(
        'https://media.example/api/media/cleanup',
        {
            method: 'POST',
            headers: { Authorization: 'Bearer staging-secret' },
        },
    ), env);
    const payload = await response.json();

    assert.equal(response.status, 200);
    assert.equal(payload.scanned, 2);
    assert.equal(payload.deleted, 1);
    assert.equal(env.MEDIA_BUCKET.objects.has('old.png'), false);
    assert.equal(env.MEDIA_BUCKET.objects.has('new.png'), true);
});

test('scheduled cleanup removes expired staged media', async () => {
    const env = testEnv();

    await env.MEDIA_BUCKET.put('scheduled-old.png', pngBytes(), {
        httpMetadata: { contentType: 'image/png' },
        customMetadata: {
            expiresAt: new Date(Date.now() - 1_000).toISOString(),
        },
    });
    await env.MEDIA_BUCKET.put('scheduled-new.png', pngBytes(), {
        httpMetadata: { contentType: 'image/png' },
        customMetadata: {
            expiresAt: new Date(Date.now() + 60_000).toISOString(),
        },
    });

    const waitUntilPromises = [];
    const ctx = {
        waitUntil(promise) {
            waitUntilPromises.push(promise);
        },
    };

    await worker.scheduled({
        cron: '0 * * * *',
        scheduledTime: Date.now(),
    }, env, ctx);
    await Promise.all(waitUntilPromises);

    assert.equal(waitUntilPromises.length, 1);
    assert.equal(env.MEDIA_BUCKET.objects.has('scheduled-old.png'), false);
    assert.equal(env.MEDIA_BUCKET.objects.has('scheduled-new.png'), true);
});

function testEnv(overrides = {}) {
    return {
        MEDIA_BUCKET: new R2BucketMock(),
        MEDIA_STAGING_TOKEN: 'staging-secret',
        PUBLIC_MEDIA_BASE_URL: 'https://media.example',
        MAX_OBJECT_BYTES: '1048576',
        DEFAULT_TTL_SECONDS: '600',
        ...overrides,
    };
}

function pngBytes() {
    return new Uint8Array([
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
        0x00, 0x00, 0x00, 0x0d,
    ]);
}

class R2BucketMock {
    constructor() {
        this.objects = new Map();
    }

    async put(key, body, options = {}) {
        const bytes = body instanceof ArrayBuffer
            ? new Uint8Array(body)
            : body instanceof Uint8Array
                ? body
                : new Uint8Array(await new Response(body).arrayBuffer());

        this.objects.set(key, new R2ObjectMock(key, bytes, options));
    }

    async get(key) {
        return this.objects.get(key) || null;
    }

    async delete(key) {
        this.objects.delete(key);
    }

    async list() {
        return {
            truncated: false,
            cursor: undefined,
            objects: [...this.objects.values()].map((object) => ({
                key: object.key,
                customMetadata: object.customMetadata,
            })),
        };
    }
}

class R2ObjectMock {
    constructor(key, bytes, options) {
        this.key = key;
        this.bytes = bytes;
        this.body = bytes;
        this.customMetadata = options.customMetadata || {};
        this.httpMetadata = options.httpMetadata || {};
        this.httpEtag = `"${key}"`;
    }

    writeHttpMetadata(headers) {
        if (this.httpMetadata.contentType) {
            headers.set('content-type', this.httpMetadata.contentType);
        }
        if (this.httpMetadata.cacheControl) {
            headers.set('cache-control', this.httpMetadata.cacheControl);
        }
    }
}
