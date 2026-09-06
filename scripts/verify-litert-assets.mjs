#!/usr/bin/env node

import { existsSync, readdirSync, statSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const requiredFiles = [
    'litert_wasm_compat_internal.js',
    'litert_wasm_compat_internal.wasm',
    'litert_wasm_internal.js',
    'litert_wasm_internal.wasm',
    'litert_wasm_jspi_internal.js',
    'litert_wasm_jspi_internal.wasm',
    'litert_wasm_threaded_internal.js',
    'litert_wasm_threaded_internal.wasm',
];
const sourceRoot = path.join(projectRoot, 'resources', 'desktop', 'public', 'litert', 'wasm');
const distRoot = path.join(projectRoot, 'resources', 'desktop', 'dist', 'litert', 'wasm');
const roots = [sourceRoot];

if (existsSync(path.join(projectRoot, 'resources', 'desktop', 'dist'))) {
    roots.push(distRoot);
}

let failed = false;

// Vite owns this generated entry, alongside the unchanged public runtime/model
// assets. A packaged app must not silently omit the background execution path.
if (roots.includes(distRoot)) {
    const builtAssets = path.resolve(distRoot, '../../assets');
    const workers = existsSync(builtAssets)
        ? readdirSync(builtAssets).filter((file) => /^localAi\.worker-[\w-]+\.js$/.test(file))
        : [];
    if (workers.length !== 1 || statSync(path.join(builtAssets, workers[0])).size === 0) {
        console.error('[missing] Expected one bundled Local AI worker');
        failed = true;
    }
}

for (const root of roots) {
    if (!existsSync(root)) {
        console.error(`[missing] ${path.relative(projectRoot, root)}`);
        failed = true;
        continue;
    }

    for (const filename of requiredFiles) {
        const file = path.join(root, filename);

        if (!existsSync(file) || statSync(file).size === 0) {
            console.error(`[missing] ${path.relative(projectRoot, file)}`);
            failed = true;
        }
    }
}

if (failed) {
    process.exit(1);
}

const modelCheck = spawnSync(process.execPath, [path.join(scriptDirectory, 'verify-local-ai-models.mjs')], {
    cwd: projectRoot,
    encoding: 'utf8',
    shell: false,
});

if (modelCheck.status !== 0) {
    process.stderr.write(modelCheck.stderr || modelCheck.stdout || 'Local AI model validation failed.\n');
    process.exit(modelCheck.status || 1);
}

console.log(
    roots.length > 1
        ? 'LiteRT runtime assets are present in public source and built desktop bundle.'
        : 'LiteRT runtime assets are present in public source.',
);
process.stdout.write(modelCheck.stdout || '');
const nativeCheck = spawnSync(process.execPath, [path.join(scriptDirectory, 'prepare-native-litert.mjs'), '--check'], { cwd: projectRoot, encoding: 'utf8' });
if (nativeCheck.status !== 0) {
    process.stderr.write(nativeCheck.stderr || nativeCheck.stdout || 'Native LiteRT validation failed.\n');
    process.exit(1);
}
process.stdout.write(nativeCheck.stdout || '');
