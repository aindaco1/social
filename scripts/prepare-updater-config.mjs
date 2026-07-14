#!/usr/bin/env node

import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { authPath, fileExists, readTrimmedFile } from './apple-auth.js';
import { gitRemoteRepoSlug } from './release-repo.js';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const checkOnly = args.includes('--check');
const requirePrivateKey = args.includes('--require-private-key');

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : null;
}

function envValue(name) {
    return String(process.env[name] || '').trim();
}

function fail(message) {
    console.error(message);
    process.exit(1);
}

function validateRepository(value) {
    if (!/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(value)) {
        fail(`Invalid release repository "${value}". Expected owner/repo.`);
    }
}

function validateEndpoint(value) {
    let url;

    try {
        url = new URL(value);
    } catch {
        fail(`Invalid updater endpoint "${value}". Expected an HTTPS URL.`);
    }

    if (url.protocol !== 'https:') {
        fail(`Invalid updater endpoint "${value}". Updater endpoints must use HTTPS.`);
    }
}

const releaseRepository = argValue('--repo')
    || envValue('DUSTWAVE_RELEASE_REPO')
    || envValue('GITHUB_REPOSITORY')
    || gitRemoteRepoSlug(projectRoot);
const publicKey = argValue('--pubkey')
    || envValue('TAURI_UPDATER_PUBLIC_KEY')
    || envValue('DUSTWAVE_TAURI_UPDATER_PUBLIC_KEY')
    || (fileExists(authPath('tauri-updater-public-key.txt'))
        ? await readTrimmedFile(authPath('tauri-updater-public-key.txt'))
        : '');
const endpoint = argValue('--endpoint')
    || envValue('DUSTWAVE_UPDATER_ENDPOINT')
    || (releaseRepository ? `https://github.com/${releaseRepository}/releases/latest/download/latest.json` : '');
const outputPath = path.resolve(
    projectRoot,
    argValue('--output')
        || envValue('DUSTWAVE_UPDATER_CONFIG_OUTPUT')
        || 'src-tauri/tauri.updater.generated.conf.json'
);

if (!releaseRepository && !endpoint) {
    fail('Missing updater release repository. Set DUSTWAVE_RELEASE_REPO=owner/repo or GITHUB_REPOSITORY.');
}

if (releaseRepository) {
    validateRepository(releaseRepository);
}

if (!publicKey || publicKey === 'REPLACE_WITH_TAURI_UPDATER_PUBLIC_KEY') {
    fail('Missing updater public key. Set TAURI_UPDATER_PUBLIC_KEY or DUSTWAVE_TAURI_UPDATER_PUBLIC_KEY.');
}

if (publicKey.length < 32) {
    fail('Updater public key looks too short. Pass the full Tauri updater public key.');
}

if (!endpoint) {
    fail('Missing updater endpoint. Set DUSTWAVE_UPDATER_ENDPOINT or DUSTWAVE_RELEASE_REPO.');
}

validateEndpoint(endpoint);

if (
    requirePrivateKey
    && !envValue('TAURI_SIGNING_PRIVATE_KEY')
    && !envValue('TAURI_SIGNING_PRIVATE_KEY_PATH')
    && !fileExists(authPath('tauri-updater-private.key'))
) {
    fail('Missing updater private key. Tauri cannot create signed updater artifacts without TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PATH, or Apple Auth tauri-updater-private.key.');
}

const config = {
    $schema: 'https://schema.tauri.app/config/2',
    bundle: {
        createUpdaterArtifacts: true,
    },
    plugins: {
        updater: {
            pubkey: publicKey,
            endpoints: [endpoint],
        },
    },
};

if (checkOnly) {
    console.log('Updater config inputs are ready.');
    console.log(`- endpoint: ${endpoint}`);
    console.log(`- output: ${path.relative(projectRoot, outputPath)}`);
    process.exit(0);
}

await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(config, null, 2)}\n`);

console.log('Prepared updater config overlay.');
console.log(`- endpoint: ${endpoint}`);
console.log(`- output: ${path.relative(projectRoot, outputPath)}`);
