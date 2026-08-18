#!/usr/bin/env node

import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const withMedia = args.includes('--media');
const withDmg = args.includes('--dmg');
const passThroughArgs = args.filter((arg) => !['--media', '--dmg'].includes(arg));
const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const npxCommand = process.platform === 'win32' ? 'npx.cmd' : 'npx';
const nodeCommand = process.execPath;
const appPath = path.join(
    projectRoot,
    'src-tauri',
    'target',
    'release',
    'bundle',
    'macos',
    'Dust Wave Social.app',
);

function run(command, commandArgs, options = {}) {
    const result = spawnSync(command, commandArgs, {
        cwd: projectRoot,
        stdio: 'inherit',
        shell: false,
        env: process.env,
        ...options,
    });

    if (result.status !== 0) {
        process.exit(result.status || 1);
    }
}

const configArgs = [];

if (withMedia) {
    run(npmCommand, ['run', 'desktop:media:check']);
    configArgs.push('--config', 'src-tauri/tauri.media-sidecars.conf.json');
}

run(npxCommand, ['tauri', 'build', ...configArgs, '--bundles', 'app', ...passThroughArgs]);
run(nodeCommand, [path.join(scriptDirectory, 'sign-macos-bundle.mjs')]);

if (withDmg && process.platform === 'darwin') {
    if (!existsSync(appPath)) {
        console.error('Expected macOS app bundle was not produced.');
        process.exit(1);
    }

    run(nodeCommand, [path.join(scriptDirectory, 'create-macos-dmg.mjs'), '--no-sign']);
} else if (withDmg) {
    console.log('Skipping local DMG creation on non-macOS host.');
}
