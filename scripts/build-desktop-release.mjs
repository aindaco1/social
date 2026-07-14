#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const withMedia = args.includes('--media');
const withUpdater = args.includes('--updater');
const tauriArgs = args.filter((arg) => arg !== '--media' && arg !== '--updater');
const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const npxCommand = process.platform === 'win32' ? 'npx.cmd' : 'npx';
const nodeCommand = process.execPath;

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

if (withUpdater) {
    run(nodeCommand, [path.join(scriptDirectory, 'prepare-updater-config.mjs'), '--require-private-key']);
    configArgs.push('--config', 'src-tauri/tauri.updater.generated.conf.json');
}

run(npxCommand, ['tauri', 'build', ...configArgs, ...tauriArgs]);

if (!withUpdater) {
    run(nodeCommand, [path.join(scriptDirectory, 'sign-macos-bundle.mjs')]);
}
