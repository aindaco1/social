#!/usr/bin/env node

import { readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { signLocalMacosCode } from './lib/macos-local-signing.mjs';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const bundleDirectory = path.join(projectRoot, 'src-tauri', 'target', 'release', 'bundle', 'macos');

function envValue(name) {
    return String(process.env[name] || '').trim();
}

function run(command, args) {
    return spawnSync(command, args, {
        cwd: projectRoot,
        encoding: 'utf8',
        stdio: 'pipe',
        shell: false,
    });
}

if (process.platform !== 'darwin') {
    console.log('Skipping macOS ad-hoc signing on non-macOS host.');
    process.exit(0);
}

if (envValue('DUSTWAVE_SKIP_ADHOC_SIGN') === 'true') {
    console.log('Skipping macOS ad-hoc signing because DUSTWAVE_SKIP_ADHOC_SIGN=true.');
    process.exit(0);
}

if (envValue('APPLE_CERTIFICATE') && !envValue('APPLE_SIGNING_IDENTITY')) {
    console.log('Skipping local signing because certificate-based signing is configured upstream.');
    process.exit(0);
}

if (!existsSync(bundleDirectory)) {
    console.log('Skipping macOS ad-hoc signing because no macOS bundle directory exists.');
    process.exit(0);
}

const apps = (await readdir(bundleDirectory, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory() && entry.name.endsWith('.app'))
    .map((entry) => path.join(bundleDirectory, entry.name));

if (!apps.length) {
    console.log('Skipping macOS ad-hoc signing because no .app bundle exists.');
    process.exit(0);
}

for (const appPath of apps) {
    const clearAttributes = run('/usr/bin/xattr', ['-cr', appPath]);

    if (clearAttributes.status !== 0) {
        console.error(clearAttributes.stderr || clearAttributes.stdout || `Failed to clear extended attributes from ${appPath}`);
        process.exit(clearAttributes.status || 1);
    }

    try {
        const signing = signLocalMacosCode(appPath, { deep: true });
        const signingLabel = signing.stable ? signing.identity : 'an ad-hoc identity';
        console.log(`Signed ${path.relative(projectRoot, appPath)} with ${signingLabel}.`);

        if (!signing.stable) {
            console.warn('This bundle will use environment-only credentials in debug builds; install a Developer ID identity for stable Keychain access.');
        }
    } catch (error) {
        console.error(error.message);
        process.exit(1);
    }
}
