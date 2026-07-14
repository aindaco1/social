#!/usr/bin/env node

import { readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

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

if (envValue('APPLE_CERTIFICATE') || envValue('APPLE_SIGNING_IDENTITY')) {
    console.log('Skipping ad-hoc signing because Apple signing variables are present.');
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
    const sign = run('/usr/bin/codesign', ['--force', '--deep', '--sign', '-', appPath]);

    if (sign.status !== 0) {
        console.error(sign.stderr || sign.stdout || `Failed to sign ${appPath}`);
        process.exit(sign.status || 1);
    }

    const verify = run('/usr/bin/codesign', ['--verify', '--deep', '--strict', '--verbose=2', appPath]);

    if (verify.status !== 0) {
        console.error(verify.stderr || verify.stdout || `Failed to verify ${appPath}`);
        process.exit(verify.status || 1);
    }

    console.log(`Ad-hoc signed ${path.relative(projectRoot, appPath)}`);
}
