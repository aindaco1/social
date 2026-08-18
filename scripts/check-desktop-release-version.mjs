#!/usr/bin/env node

import { readFileSync } from 'node:fs';

const args = process.argv.slice(2);

function argValue(name) {
    const index = args.indexOf(name);
    return index >= 0 ? args[index + 1] : '';
}

function readJson(filePath) {
    return JSON.parse(readFileSync(filePath, 'utf8'));
}

function cargoVersion(filePath) {
    const cargoToml = readFileSync(filePath, 'utf8');
    return cargoToml.split(/\n\[/, 1)[0].match(/^version\s*=\s*"([^"]+)"/m)?.[1] || '';
}

const packageJson = readJson('package.json');
const packageLock = readJson('package-lock.json');
const tauriConfig = readJson('src-tauri/tauri.conf.json');
const versions = {
    'package.json': packageJson.version,
    'package-lock.json': packageLock.version,
    'package-lock.json root package': packageLock.packages?.['']?.version,
    'src-tauri/Cargo.toml': cargoVersion('src-tauri/Cargo.toml'),
    'src-tauri/tauri.conf.json': tauriConfig.version,
};
const expectedVersion = packageJson.version;

if (!expectedVersion || !/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(expectedVersion)) {
    console.error(`Desktop version must be SemVer-compatible, got ${expectedVersion || '(missing)'}.`);
    process.exit(1);
}

const mismatched = Object.entries(versions).filter(([, version]) => version !== expectedVersion);

if (mismatched.length) {
    console.error(`Desktop release version metadata must match package.json (${expectedVersion}).`);
    for (const [filePath, version] of mismatched) {
        console.error(`- ${filePath}: ${version || '(missing)'}`);
    }
    process.exit(1);
}

const expectedTag = `v${expectedVersion}`;
const releaseTag = argValue('--tag') || String(process.env.DUSTWAVE_RELEASE_TAG || '').trim();

if (releaseTag && releaseTag !== expectedTag) {
    console.error(`Release tag ${releaseTag} does not match desktop version ${expectedVersion}; expected ${expectedTag}.`);
    process.exit(1);
}

console.log(`Desktop release version metadata is synchronized for ${expectedVersion} (${expectedTag}).`);
