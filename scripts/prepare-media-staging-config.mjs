#!/usr/bin/env node

import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const args = new Set(process.argv.slice(2));
const check = args.has('--check');
const print = args.has('--print');
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const workerDirectory = path.join(projectRoot, 'workers', 'media-staging');
const templateConfigPath = path.join(workerDirectory, 'wrangler.toml');
const generatedConfigPath = path.join(workerDirectory, 'wrangler.generated.jsonc');

function firstEnv(...names) {
    for (const name of names) {
        const value = String(process.env[name] || '').trim();

        if (value) {
            return value;
        }
    }

    return '';
}

async function readTemplateBucketName() {
    try {
        const template = await readFile(templateConfigPath, 'utf8');
        const match = template.match(/bucket_name\s*=\s*"([^"]+)"/);

        return match?.[1] || '';
    } catch {
        return '';
    }
}

function parseBoolean(value, fallback) {
    const normalized = String(value || '').trim().toLowerCase();

    if (!normalized) {
        return fallback;
    }

    return ['1', 'true', 'yes', 'on'].includes(normalized);
}

function cronSchedules(value) {
    const raw = String(value || '0 * * * *').trim();
    const schedules = raw
        .split(',')
        .map((schedule) => schedule.trim())
        .filter(Boolean);

    return schedules.length > 0 ? schedules : ['0 * * * *'];
}

const bucketName = firstEnv('MEDIA_STAGING_BUCKET_NAME', 'DUSTWAVE_MEDIA_STAGING_BUCKET_NAME')
    || await readTemplateBucketName()
    || 'dustwave-media-staging';
const publicBaseUrl = firstEnv('PUBLIC_MEDIA_BASE_URL', 'MEDIA_STAGING_PUBLIC_BASE_URL')
    || 'https://dustwave-media-staging.jogo.workers.dev';
const cleanupCron = firstEnv('MEDIA_STAGING_CLEANUP_CRON');
const config = {
    $schema: 'node_modules/wrangler/config-schema.json',
    name: firstEnv('MEDIA_STAGING_WORKER_NAME', 'DUSTWAVE_MEDIA_STAGING_WORKER_NAME')
        || 'dustwave-media-staging',
    main: 'src/index.js',
    compatibility_date: firstEnv('MEDIA_STAGING_COMPATIBILITY_DATE')
        || '2026-07-15',
    workers_dev: parseBoolean(firstEnv('MEDIA_STAGING_WORKERS_DEV'), true),
    triggers: {
        crons: cronSchedules(cleanupCron),
    },
    vars: {
        PUBLIC_MEDIA_BASE_URL: publicBaseUrl.replace(/\/+$/, ''),
        MAX_OBJECT_BYTES: firstEnv('MEDIA_STAGING_MAX_OBJECT_BYTES') || '26214400',
        DEFAULT_TTL_SECONDS: firstEnv('MEDIA_STAGING_DEFAULT_TTL_SECONDS') || '86400',
    },
    r2_buckets: [
        {
            binding: 'MEDIA_BUCKET',
            bucket_name: bucketName,
        },
    ],
    secrets: {
        required: ['MEDIA_STAGING_TOKEN'],
    },
};

await mkdir(workerDirectory, { recursive: true });
const generated = `${JSON.stringify(config, null, 4)}\n`;
await writeFile(generatedConfigPath, generated);

const relativeGeneratedPath = path.relative(projectRoot, generatedConfigPath);
const missing = [];

if (!bucketName) {
    missing.push('MEDIA_STAGING_BUCKET_NAME');
}

if (!publicBaseUrl.startsWith('https://')) {
    missing.push('PUBLIC_MEDIA_BASE_URL must be HTTPS');
}

if (print) {
    console.log(generated);
} else {
    console.log(`Wrote ${relativeGeneratedPath}`);
}

if (missing.length > 0) {
    const message = `Missing media staging deploy setting: ${missing.join(', ')}`;

    if (check) {
        console.error(message);
        process.exit(1);
    }

    console.warn(`${message}. Config is usable for local scaffolding only.`);
}
