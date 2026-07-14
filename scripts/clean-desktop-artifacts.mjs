#!/usr/bin/env node

import { existsSync } from 'node:fs';
import { rm } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const dryRun = process.argv.includes('--dry-run');

const artifactPaths = [
    'resources/dist',
    'resources/desktop/dist',
    'src-tauri/gen',
    'src-tauri/target',
    'src-tauri/tauri.macos-signing.generated.conf.json',
    'src-tauri/tauri.updater.generated.conf.json',
];

for (const relativePath of artifactPaths) {
    const absolutePath = path.join(projectRoot, relativePath);

    if (!existsSync(absolutePath)) {
        continue;
    }

    if (dryRun) {
        console.log(`[dry-run] remove ${relativePath}`);
        continue;
    }

    await rm(absolutePath, {
        force: true,
        maxRetries: 8,
        recursive: true,
        retryDelay: 250,
    });
    console.log(`Removed ${relativePath}`);
}

if (!dryRun) {
    console.log('Desktop build artifact cleanup complete.');
}
