#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { gitRemoteRepoSlug } from './release-repo.js';

const args = process.argv.slice(2);
const apply = args.includes('--apply');
const repoArgIndex = args.findIndex((arg) => arg === '--repo');
const envFileArgIndex = args.findIndex((arg) => arg === '--env-file');
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const repo =
    (repoArgIndex >= 0 ? args[repoArgIndex + 1] : '') ||
    process.env.DUSTWAVE_RELEASE_REPO ||
    process.env.GITHUB_REPOSITORY ||
    gitRemoteRepoSlug(projectRoot) ||
    '';

const envFilePath = envFileArgIndex >= 0 ? args[envFileArgIndex + 1] : '';
const envFromFile = envFilePath ? await parseEnvFile(envFilePath) : {};

const secretNames = [
    'CLOUDFLARE_API_TOKEN',
    'CLOUDFLARE_ACCOUNT_ID',
    'MEDIA_STAGING_TOKEN',
];

const variableNames = [
    'MEDIA_STAGING_WORKER_NAME',
    'MEDIA_STAGING_BUCKET_NAME',
    'PUBLIC_MEDIA_BASE_URL',
    'MEDIA_STAGING_MAX_OBJECT_BYTES',
    'MEDIA_STAGING_DEFAULT_TTL_SECONDS',
    'MEDIA_STAGING_CLEANUP_CRON',
];

function run(command, commandArgs, options = {}) {
    return spawnSync(command, commandArgs, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        input: options.input,
        stdio: options.input ? ['pipe', 'pipe', 'pipe'] : 'pipe',
    });
}

function requireRepo() {
    if (!repo.trim()) {
        console.error('Pass --repo owner/repo or set DUSTWAVE_RELEASE_REPO.');
        process.exit(1);
    }
}

function ghSettingNames(kind) {
    const result = run('gh', [kind, 'list', '--repo', repo]);

    if (result.status !== 0) {
        console.warn(String(result.stderr || result.stdout || `unable to list GitHub ${kind}s`).trim());

        return new Set();
    }

    return new Set(
        String(result.stdout || '')
            .split(/\r?\n/)
            .map((line) => line.split(/\s+/)[0])
            .filter(Boolean),
    );
}

async function parseEnvFile(filePath) {
    const text = await readFile(filePath, 'utf8');
    const values = {};

    for (const line of text.split(/\r?\n/)) {
        const trimmed = line.trim();

        if (!trimmed || trimmed.startsWith('#')) {
            continue;
        }

        const match = trimmed.match(/^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/);

        if (!match) {
            continue;
        }

        values[match[1]] = unquote(match[2].trim());
    }

    return values;
}

function unquote(value) {
    if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
    ) {
        return value.slice(1, -1);
    }

    return value;
}

function valueFor(name) {
    return String(process.env[name] || envFromFile[name] || '').trim();
}

function planned(kind, name) {
    const action = apply ? 'set' : 'would set';
    console.log(`${action} ${kind} ${name}`);
}

function setSecret(name, value) {
    planned('secret', name);

    if (!apply) {
        return;
    }

    const result = run('gh', ['secret', 'set', name, '--repo', repo], {
        input: value,
    });

    if (result.status !== 0) {
        console.error(result.stderr || result.stdout || `Failed to set secret ${name}.`);
        process.exit(result.status || 1);
    }
}

function setVariable(name, value) {
    planned('variable', name);

    if (!apply) {
        return;
    }

    const result = run('gh', ['variable', 'set', name, '--repo', repo], {
        input: value,
    });

    if (result.status !== 0) {
        console.error(result.stderr || result.stdout || `Failed to set variable ${name}.`);
        process.exit(result.status || 1);
    }
}

requireRepo();

const existingSecrets = ghSettingNames('secret');
const existingVariables = ghSettingNames('variable');

console.log(`${apply ? 'Applying' : 'Planning'} GitHub media staging settings for ${repo}`);

for (const name of secretNames) {
    const value = valueFor(name);

    if (value) {
        setSecret(name, value);
    } else if (existingSecrets.has(name)) {
        console.log(`already set secret ${name}`);
    } else {
        console.warn(`missing ${name}`);
    }
}

for (const name of variableNames) {
    const value = valueFor(name);

    if (value) {
        setVariable(name, value);
    } else if (existingVariables.has(name)) {
        console.log(`already set variable ${name}`);
    }
}

if (!apply) {
    console.log('Plan only. Re-run with --apply to write GitHub secrets and variables.');
}
