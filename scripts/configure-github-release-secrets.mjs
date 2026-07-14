#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
    appleApiKeyIdFromPath,
    appleAuthDirectory,
    authPath,
    fileExists,
    readTrimmedFile,
    redactPath,
} from './apple-auth.js';
import { gitRemoteRepoSlug } from './release-repo.js';

const args = process.argv.slice(2);
const apply = args.includes('--apply');
const repoArgIndex = args.findIndex((arg) => arg === '--repo');
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const repo =
    (repoArgIndex >= 0 ? args[repoArgIndex + 1] : '') ||
    process.env.DUSTWAVE_RELEASE_REPO ||
    process.env.GITHUB_REPOSITORY ||
    gitRemoteRepoSlug(projectRoot) ||
    '';

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

function codesignIdentity() {
    const result = run('security', ['find-identity', '-p', 'codesigning', '-v']);
    const output = `${result.stdout || ''}\n${result.stderr || ''}`;
    const match = output.match(/"((?:Developer ID Application:)[^"]+)"/);

    return match?.[1] || '';
}

function teamIdFromIdentity(identity) {
    return identity.match(/\(([A-Z0-9]{10})\)$/)?.[1] || '';
}

async function firstAppleApiKeyPath() {
    const directory = appleAuthDirectory();

    if (!fileExists(directory)) {
        return '';
    }

    const entries = await readdir(directory);
    const key = entries.find((entry) => /^AuthKey_[A-Z0-9]+\.p8$/.test(entry));

    return key ? path.join(directory, key) : '';
}

async function optionalIssuer() {
    for (const fileName of [
        'apple-api-issuer.txt',
        'app-store-connect-issuer.txt',
        'issuer.txt',
    ]) {
        const filePath = authPath(fileName);

        if (fileExists(filePath)) {
            return readTrimmedFile(filePath);
        }
    }

    return String(process.env.APPLE_API_ISSUER || '').trim();
}

function planned(kind, name, detail = '') {
    const action = apply ? 'set' : 'would set';
    console.log(`${action} ${kind} ${name}${detail ? ` (${detail})` : ''}`);
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

const certificatePath = authPath('developer-id-application.p12');
const certificatePasswordPath = authPath('apple-p12-password.txt');
const apiKeyPath = await firstAppleApiKeyPath();
const apiKeyId = appleApiKeyIdFromPath(apiKeyPath);
const issuer = await optionalIssuer();
const updaterPrivateKeyPath = authPath('tauri-updater-private.key');
const updaterPasswordPath = authPath('tauri-updater-password.txt');
const updaterPublicKeyPath = authPath('tauri-updater-public-key.txt');
const identity = codesignIdentity();
const teamId = teamIdFromIdentity(identity);

console.log(`${apply ? 'Applying' : 'Planning'} GitHub release credentials for ${repo}`);
console.log(`Apple Auth: ${redactPath(appleAuthDirectory())}`);

if (fileExists(certificatePath)) {
    setSecret('APPLE_CERTIFICATE', (await readFile(certificatePath)).toString('base64'));
} else {
    console.warn('missing developer-id-application.p12');
}

if (fileExists(certificatePasswordPath)) {
    setSecret('APPLE_CERTIFICATE_PASSWORD', await readTrimmedFile(certificatePasswordPath));
} else {
    console.warn('missing apple-p12-password.txt');
}

if (identity) {
    setSecret('APPLE_SIGNING_IDENTITY', identity);
}

if (teamId) {
    setSecret('APPLE_TEAM_ID', teamId);
    setSecret('APPLE_PROVIDER_SHORT_NAME', teamId);
}

if (apiKeyPath && apiKeyId) {
    setSecret('APPLE_API_KEY', apiKeyId);
    setSecret('APPLE_API_KEY_P8', await readFile(apiKeyPath, 'utf8'));
} else {
    console.warn('missing AuthKey_*.p8');
}

if (issuer) {
    setSecret('APPLE_API_ISSUER', issuer);
} else {
    console.warn('missing Apple API issuer; omit only if this is an Individual API key');
}

if (fileExists(updaterPrivateKeyPath) && fileExists(updaterPasswordPath)) {
    setSecret('TAURI_SIGNING_PRIVATE_KEY', await readFile(updaterPrivateKeyPath, 'utf8'));
    setSecret('TAURI_SIGNING_PRIVATE_KEY_PASSWORD', await readTrimmedFile(updaterPasswordPath));
} else {
    console.warn('missing updater private key/password; run npm run desktop:updater:keys');
}

if (fileExists(updaterPublicKeyPath)) {
    setVariable('TAURI_UPDATER_PUBLIC_KEY', await readTrimmedFile(updaterPublicKeyPath));
    setVariable('DUSTWAVE_RELEASE_REPO', repo);
} else {
    console.warn('missing updater public key; run npm run desktop:updater:keys');
}

if (!apply) {
    console.log('Plan only. Re-run with --apply to write GitHub secrets and variables.');
}
