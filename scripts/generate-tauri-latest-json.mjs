#!/usr/bin/env node

import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { gitRemoteRepoSlug } from './release-repo.js';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : '';
}

function envValue(name) {
    return String(process.env[name] || '').trim();
}

function fail(message) {
    console.error(message);
    process.exit(1);
}

function hostPlatformKey() {
    const arch = process.arch === 'arm64' ? 'aarch64' : process.arch === 'x64' ? 'x86_64' : process.arch;

    if (process.platform === 'darwin') {
        return `darwin-${arch}`;
    }

    if (process.platform === 'win32') {
        return `windows-${arch}`;
    }

    return `linux-${arch}`;
}

async function tauriVersion() {
    const configPath = path.join(projectRoot, 'src-tauri', 'tauri.conf.json');
    const config = JSON.parse(await readFile(configPath, 'utf8'));

    return String(config.version || '0.1.0');
}

const releaseRepository = argValue('--repo')
    || envValue('DUSTWAVE_RELEASE_REPO')
    || envValue('GITHUB_REPOSITORY')
    || gitRemoteRepoSlug(projectRoot);
const platform = argValue('--platform') || hostPlatformKey();
const version = argValue('--version') || await tauriVersion();
const artifactPath = path.resolve(
    projectRoot,
    argValue('--artifact') || 'src-tauri/target/release/bundle/macos/Dust Wave Social.app.tar.gz',
);
const signaturePath = path.resolve(projectRoot, argValue('--signature') || `${artifactPath}.sig`);
const outputPath = path.resolve(
    projectRoot,
    argValue('--output') || 'src-tauri/target/release/bundle/latest.json',
);
const notes = argValue('--notes') || envValue('DUSTWAVE_RELEASE_NOTES') || `Dust Wave Social ${version}`;

if (!releaseRepository) {
    fail('Missing release repository. Set DUSTWAVE_RELEASE_REPO=owner/repo or use --repo owner/repo.');
}

if (!existsSync(artifactPath)) {
    fail(`Missing updater artifact: ${path.relative(projectRoot, artifactPath)}`);
}

if (!existsSync(signaturePath)) {
    fail(`Missing updater signature: ${path.relative(projectRoot, signaturePath)}`);
}

const signature = (await readFile(signaturePath, 'utf8')).trim();

if (!signature) {
    fail(`Updater signature is empty: ${path.relative(projectRoot, signaturePath)}`);
}

const artifactName = path.basename(artifactPath);
const manifest = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms: {
        [platform]: {
            signature,
            url: `https://github.com/${releaseRepository}/releases/latest/download/${encodeURIComponent(artifactName)}`,
        },
    },
};

await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);

console.log(`Generated latest.json for ${platform}.`);
console.log(`- url: ${manifest.platforms[platform].url}`);
console.log(`- output: ${path.relative(projectRoot, outputPath)}`);
