#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdir, rm, symlink } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { readFile } from 'node:fs/promises';
import { redactPath } from './apple-auth.js';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const appName = 'Dust Wave Social.app';
const defaultAppPath = path.join(projectRoot, 'src-tauri', 'target', 'release', 'bundle', 'macos', appName);

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : '';
}

function run(command, commandArgs, options = {}) {
    return spawnSync(command, commandArgs, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: options.stdio || 'pipe',
        env: options.env || process.env,
    });
}

function fail(message) {
    console.error(message);
    process.exit(1);
}

function codesignIdentity() {
    const configured = String(argValue('--identity') || process.env.APPLE_SIGNING_IDENTITY || '').trim();

    if (configured) {
        return configured;
    }

    const result = run('security', ['find-identity', '-p', 'codesigning', '-v']);
    const output = `${result.stdout || ''}\n${result.stderr || ''}`;
    const match = output.match(/"((?:Developer ID Application:)[^"]+)"/);

    return match?.[1] || '';
}

function hostArchSuffix() {
    const result = run('rustc', ['--print', 'host-tuple']);
    const triple = String(result.stdout || '').trim();

    if (triple.startsWith('aarch64-')) {
        return 'aarch64';
    }

    if (triple.startsWith('x86_64-')) {
        return 'x64';
    }

    return process.arch === 'arm64' ? 'aarch64' : process.arch;
}

async function tauriVersion() {
    const configPath = path.join(projectRoot, 'src-tauri', 'tauri.conf.json');
    const config = JSON.parse(await readFile(configPath, 'utf8'));

    return String(config.version || '0.1.0');
}

const appPath = path.resolve(projectRoot, argValue('--app') || defaultAppPath);
const version = argValue('--version') || await tauriVersion();
const outputPath = path.resolve(
    projectRoot,
    argValue('--out') || path.join(
        'src-tauri',
        'target',
        'release',
        'bundle',
        'dmg',
        `Dust Wave Social_${version}_${hostArchSuffix()}.dmg`,
    ),
);
const volumeName = argValue('--volname') || 'Dust Wave Social';
const shouldSign = !args.includes('--no-sign');

if (process.platform !== 'darwin') {
    fail('DMG packaging requires macOS.');
}

if (!existsSync(appPath)) {
    fail(`App bundle not found at ${redactPath(appPath)}. Build the signed app first.`);
}

const stagingDirectory = path.join(os.tmpdir(), `dustwave-dmg-${process.pid}-${Date.now()}`);

await rm(stagingDirectory, { force: true, recursive: true });
await mkdir(stagingDirectory, { recursive: true });
await mkdir(path.dirname(outputPath), { recursive: true });
await rm(outputPath, { force: true });

const stagedAppPath = path.join(stagingDirectory, appName);
let result = run('ditto', [appPath, stagedAppPath], { stdio: 'inherit' });

if (result.status !== 0) {
    await rm(stagingDirectory, { force: true, recursive: true });
    process.exit(result.status || 1);
}

await symlink('/Applications', path.join(stagingDirectory, 'Applications'));

console.log(`Creating DMG: ${redactPath(outputPath)}`);
result = run('hdiutil', [
    'create',
    '-volname',
    volumeName,
    '-srcfolder',
    stagingDirectory,
    '-ov',
    '-format',
    'UDZO',
    outputPath,
], { stdio: 'inherit' });

await rm(stagingDirectory, { force: true, recursive: true });

if (result.status !== 0) {
    process.exit(result.status || 1);
}

if (shouldSign) {
    const identity = codesignIdentity();

    if (!identity) {
        fail('No Developer ID Application signing identity is available for DMG signing.');
    }

    console.log(`Signing DMG with identity: ${identity}`);
    result = run('codesign', ['--force', '--sign', identity, '--timestamp', outputPath], { stdio: 'inherit' });

    if (result.status !== 0) {
        process.exit(result.status || 1);
    }

    result = run('codesign', ['--verify', '--verbose=2', outputPath], { stdio: 'inherit' });

    if (result.status !== 0) {
        process.exit(result.status || 1);
    }
}

console.log(`Created DMG: ${redactPath(outputPath)}`);
