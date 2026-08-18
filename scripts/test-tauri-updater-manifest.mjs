#!/usr/bin/env node

import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const root = await mkdtemp(path.join(os.tmpdir(), 'dust-wave-updater-manifest-'));
const artifactPath = path.join(root, 'Dust Wave Social.app.tar.gz');
const signaturePath = `${artifactPath}.sig`;
const outputPath = path.join(root, 'latest.json');

try {
    await mkdir(root, { recursive: true });
    await writeFile(artifactPath, 'archive');
    await writeFile(signaturePath, 'signed-base64\n');
    execFileSync(process.execPath, [
        path.join(scriptDirectory, 'generate-tauri-latest-json.mjs'),
        '--repo',
        'aindaco1/social',
        '--tag',
        'v0.1.0',
        '--version',
        '0.1.0',
        '--platform',
        'darwin-aarch64',
        '--artifact',
        artifactPath,
        '--signature',
        signaturePath,
        '--output',
        outputPath,
    ], { stdio: 'pipe' });

    const manifest = JSON.parse(await readFile(outputPath, 'utf8'));
    assert.equal(manifest.version, '0.1.0');
    assert.equal(manifest.platforms['darwin-aarch64'].signature, 'signed-base64');
    assert.equal(
        manifest.platforms['darwin-aarch64'].url,
        'https://github.com/aindaco1/social/releases/download/v0.1.0/Dust.Wave.Social.app.tar.gz',
    );
    assert.equal(Number.isNaN(Date.parse(manifest.pub_date)), false);
    console.log('Tauri updater manifest tests passed.');
} finally {
    await rm(root, { recursive: true, force: true });
}
