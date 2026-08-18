#!/usr/bin/env node

import { existsSync } from 'node:fs';
import { lstat, mkdir, readlink } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const projectTargetPath = path.join(projectRoot, 'src-tauri', 'target');
const safeTargetPath = path.join(os.homedir(), 'Library', 'Caches', 'DustWaveSocial', 'target');

if (process.platform !== 'darwin' || process.env.DUSTWAVE_RELEASE_USE_PROJECT_TARGET === 'true') {
    process.exit(0);
}

let targetMetadata = null;

try {
    targetMetadata = await lstat(projectTargetPath);
} catch {
    targetMetadata = null;
}

if (!targetMetadata?.isSymbolicLink() || existsSync(projectTargetPath)) {
    process.exit(0);
}

const linkValue = await readlink(projectTargetPath);
const resolvedLink = path.resolve(path.dirname(projectTargetPath), linkValue);

if (resolvedLink !== safeTargetPath) {
    console.error(`Refusing to repair unexpected broken target symlink: ${projectTargetPath} -> ${resolvedLink}`);
    process.exit(1);
}

await mkdir(safeTargetPath, { recursive: true });
console.log('Recreated the cache-backed desktop target directory.');
