#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { chmod, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { authPath, fileExists, readTrimmedFile, redactPath } from './apple-auth.js';

const force = process.argv.includes('--force');
const privateKeyPath = authPath('tauri-updater-private.key');
const passwordPath = authPath('tauri-updater-password.txt');
const publicKeyPath = `${privateKeyPath}.pub`;
const publicKeyCopyPath = authPath('tauri-updater-public-key.txt');
const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');

function run(command, args) {
    return spawnSync(command, args, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
    });
}

await mkdir(path.dirname(privateKeyPath), { recursive: true });

if (fileExists(privateKeyPath) && !force) {
    console.log(`Updater private key already exists at ${redactPath(privateKeyPath)}`);

    if (fileExists(publicKeyPath) && !fileExists(publicKeyCopyPath)) {
        await writeFile(publicKeyCopyPath, await readFile(publicKeyPath, 'utf8'), { mode: 0o600 });
    }

    if (fileExists(publicKeyCopyPath)) {
        console.log(`Updater public key is available at ${redactPath(publicKeyCopyPath)}`);
    }

    process.exit(0);
}

const password = fileExists(passwordPath)
    ? await readTrimmedFile(passwordPath)
    : randomBytes(32).toString('base64url');

if (!fileExists(passwordPath) || force) {
    await writeFile(passwordPath, `${password}\n`, { mode: 0o600 });
}

const result = run('npx', [
    'tauri',
    'signer',
    'generate',
    '--ci',
    '--password',
    password,
    '--write-keys',
    privateKeyPath,
    ...(force ? ['--force'] : []),
]);

if (result.status !== 0) {
    console.error(result.stderr || result.stdout || 'Failed to generate updater signing key.');
    process.exit(result.status || 1);
}

if (!fileExists(publicKeyPath)) {
    console.error('Tauri signer did not write the expected public key file.');
    process.exit(1);
}

await chmod(privateKeyPath, 0o600);
await writeFile(publicKeyCopyPath, await readFile(publicKeyPath, 'utf8'), { mode: 0o600 });

console.log(`Updater private key written to ${redactPath(privateKeyPath)}`);
console.log(`Updater password written to ${redactPath(passwordPath)}`);
console.log(`Updater public key written to ${redactPath(publicKeyCopyPath)}`);
