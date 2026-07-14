#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { readdir, rm } from 'node:fs/promises';
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

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const appArgIndex = args.indexOf('--app');
const pathArgIndex = args.indexOf('--path');
const dmgArgIndex = args.indexOf('--dmg');
const submissionIdArgIndex = args.indexOf('--submission-id');
const submissionId = submissionIdArgIndex >= 0 ? args[submissionIdArgIndex + 1] : '';
const statusOnly = args.includes('--status') || args.includes('--info');
const timeoutArgIndex = args.indexOf('--timeout');
const waitTimeout = timeoutArgIndex >= 0 ? args[timeoutArgIndex + 1] : '30m';
const defaultAppPath = path.resolve(
    projectRoot,
    'src-tauri/target/release/bundle/macos/Dust Wave Social.app',
);
const defaultDmgPath = path.resolve(
    projectRoot,
    'src-tauri/target/release/bundle/dmg/Dust Wave Social_0.1.0_aarch64.dmg',
);
const artifactPath = path.resolve(
    projectRoot,
    pathArgIndex >= 0
        ? args[pathArgIndex + 1]
        : dmgArgIndex >= 0
            ? args[dmgArgIndex + 1]
            : appArgIndex >= 0
                ? args[appArgIndex + 1]
                : fileExists(defaultDmgPath)
                    ? defaultDmgPath
                    : defaultAppPath,
);
const isAppBundle = artifactPath.endsWith('.app');
const submitPath = isAppBundle ? `${artifactPath}.zip` : artifactPath;

function run(command, commandArgs, options = {}) {
    return spawnSync(command, commandArgs, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: options.stdio || 'pipe',
    });
}

function fail(message) {
    console.error(message);
    process.exit(1);
}

function authArgs() {
    return [
        '--key',
        keyPath,
        '--key-id',
        keyId,
        '--issuer',
        issuer,
    ];
}

function parseSubmissionId(output) {
    try {
        const data = JSON.parse(output);

        return String(data.id || '').trim();
    } catch {
        const match = output.match(/\bid:\s*([0-9a-fA-F-]{36})\b/);

        return match?.[1] || '';
    }
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

async function appleApiIssuer() {
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

if (!fileExists(artifactPath)) {
    fail(`Notarization artifact not found at ${redactPath(artifactPath)}. Build a signed app or DMG first.`);
}

const keyPath = String(process.env.APPLE_API_KEY_PATH || '').trim() || (await firstAppleApiKeyPath());
const keyId = String(process.env.APPLE_API_KEY || '').trim() || appleApiKeyIdFromPath(keyPath);
const issuer = await appleApiIssuer();

if (!keyPath || !keyId) {
    fail(`No AuthKey_*.p8 file found in ${redactPath(appleAuthDirectory())}.`);
}

if (!issuer) {
    fail('Missing Apple API issuer UUID. Add it to Apple Auth as apple-api-issuer.txt or set APPLE_API_ISSUER.');
}

let result;
let currentSubmissionId = submissionId;

if (statusOnly) {
    if (!currentSubmissionId) {
        fail('Pass --submission-id <id> when checking notarization status.');
    }

    result = run('xcrun', [
        'notarytool',
        'info',
        currentSubmissionId,
        ...authArgs(),
        '--output-format',
        'json',
    ]);

    if (result.status !== 0) {
        process.stderr.write(result.stderr || result.stdout || 'Failed to read notarization status.');
        process.exit(result.status || 1);
    }

    const data = JSON.parse(result.stdout);
    console.log(JSON.stringify({
        id: data.id,
        name: data.name,
        status: data.status,
        createdDate: data.createdDate,
    }, null, 2));
    process.exit(0);
}

if (isAppBundle && !currentSubmissionId) {
    console.log(`Creating notarization zip: ${redactPath(submitPath)}`);
    await rm(submitPath, { force: true });
    result = run('ditto', ['-c', '-k', '--keepParent', artifactPath, submitPath], { stdio: 'inherit' });

    if (result.status !== 0) {
        process.exit(result.status || 1);
    }
}

if (!currentSubmissionId) {
    console.log(`Submitting ${isAppBundle ? 'app' : 'DMG'} for notarization.`);
    result = run('xcrun', [
        'notarytool',
        'submit',
        submitPath,
        ...authArgs(),
        '--output-format',
        'json',
    ]);

    if (result.status !== 0) {
        process.stderr.write(result.stderr || result.stdout || 'Failed to submit artifact for notarization.');
        process.exit(result.status || 1);
    }

    currentSubmissionId = parseSubmissionId(result.stdout);

    if (!currentSubmissionId) {
        process.stdout.write(result.stdout || '');
        fail('Notarization submission succeeded but no submission ID was found.');
    }

    console.log(`Notarization submission ID: ${currentSubmissionId}`);
}

console.log(`Waiting for notarization submission: ${currentSubmissionId}`);
result = run('xcrun', [
    'notarytool',
    'wait',
    currentSubmissionId,
    ...authArgs(),
    '--timeout',
    waitTimeout,
], { stdio: 'inherit' });

if (result.status !== 0) {
    process.exit(result.status || 1);
}

console.log('Stapling notarization ticket.');
result = run('xcrun', ['stapler', 'staple', artifactPath], { stdio: 'inherit' });

if (result.status !== 0) {
    process.exit(result.status || 1);
}

console.log('Validating Gatekeeper assessment.');
result = run('spctl', ['-a', '-vv', artifactPath], { stdio: 'inherit' });

if (result.status !== 0) {
    process.exit(result.status || 1);
}
