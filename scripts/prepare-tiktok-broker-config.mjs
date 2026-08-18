#!/usr/bin/env node

import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const args = new Set(process.argv.slice(2));
const check = args.has('--check');
const print = args.has('--print');
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const workerDirectory = path.join(projectRoot, 'workers', 'tiktok-broker');
const templateConfigPath = path.join(workerDirectory, 'wrangler.toml');
const generatedConfigPath = path.join(workerDirectory, 'wrangler.generated.jsonc');
const defaultScopes = 'user.info.basic,user.info.stats,video.list';
const placeholderDatabaseId = 'replace-with-d1-database-id';

function firstEnv(...names) {
    for (const name of names) {
        const value = String(process.env[name] || '').trim();

        if (value) {
            return value;
        }
    }

    return '';
}

async function readTemplateDatabaseId() {
    try {
        const template = await readFile(templateConfigPath, 'utf8');
        const match = template.match(/database_id\s*=\s*"([^"]+)"/);

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

function isValidD1DatabaseId(value) {
    return /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(value);
}

const templateDatabaseId = await readTemplateDatabaseId();
const databaseId = firstEnv(
    'TIKTOK_BROKER_D1_DATABASE_ID',
    'DUSTWAVE_TIKTOK_BROKER_D1_DATABASE_ID',
) || (templateDatabaseId && templateDatabaseId !== placeholderDatabaseId ? templateDatabaseId : '');

const config = {
    $schema: 'node_modules/wrangler/config-schema.json',
    name: firstEnv('TIKTOK_BROKER_WORKER_NAME', 'DUSTWAVE_TIKTOK_BROKER_WORKER_NAME')
        || 'dustwave-tiktok-broker',
    main: 'src/index.js',
    compatibility_date: firstEnv('TIKTOK_BROKER_COMPATIBILITY_DATE')
        || '2026-07-14',
    workers_dev: parseBoolean(firstEnv('TIKTOK_BROKER_WORKERS_DEV'), true),
    vars: {
        TIKTOK_SCOPES: firstEnv('TIKTOK_SCOPES') || defaultScopes,
    },
    d1_databases: [
        {
            binding: 'DB',
            database_name: firstEnv(
                'TIKTOK_BROKER_D1_DATABASE_NAME',
                'DUSTWAVE_TIKTOK_BROKER_D1_DATABASE_NAME',
            ) || 'dustwave-tiktok-broker',
            database_id: databaseId || placeholderDatabaseId,
        },
    ],
};

const publicBrokerBaseUrl = firstEnv('PUBLIC_BROKER_BASE_URL', 'TIKTOK_BROKER_PUBLIC_BASE_URL');

if (publicBrokerBaseUrl) {
    config.vars.PUBLIC_BROKER_BASE_URL = publicBrokerBaseUrl.replace(/\/+$/, '');
}

await mkdir(workerDirectory, { recursive: true });
const generated = `${JSON.stringify(config, null, 4)}\n`;
await writeFile(generatedConfigPath, generated);

const relativeGeneratedPath = path.relative(projectRoot, generatedConfigPath);
const missing = [];

if (!databaseId || databaseId === placeholderDatabaseId) {
    missing.push('TIKTOK_BROKER_D1_DATABASE_ID');
} else if (!isValidD1DatabaseId(databaseId)) {
    missing.push('TIKTOK_BROKER_D1_DATABASE_ID must be a D1 UUID');
}

if (print) {
    console.log(generated);
} else {
    console.log(`Wrote ${relativeGeneratedPath}`);
}

if (missing.length > 0) {
    const message = `Missing broker deploy setting: ${missing.join(', ')}`;

    if (check) {
        console.error(message);
        process.exit(1);
    }

    console.warn(`${message}. Config is usable for local scaffolding only.`);
}
