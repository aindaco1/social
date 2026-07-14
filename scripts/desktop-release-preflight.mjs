#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
    appleApiKeyIdFromPath,
    appleAuthDirectory,
    authPath,
    redactPath,
} from './apple-auth.js';
import { gitRemoteRepoSlug } from './release-repo.js';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const strict = process.argv.includes('--strict');
const nodeCommand = process.execPath;
const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';
const npxCommand = process.platform === 'win32' ? 'npx.cmd' : 'npx';
const checks = [];

function run(command, args, options = {}) {
    return spawnSync(command, args, {
        cwd: projectRoot,
        encoding: 'utf8',
        stdio: options.stdio || 'pipe',
        shell: false,
        env: process.env,
    });
}

function record(status, label, detail = '') {
    checks.push({ status, label, detail });
}

function commandExists(command) {
    const lookup = process.platform === 'win32' ? 'where' : 'which';
    return run(lookup, [command]).status === 0;
}

function envAny(names) {
    return names.some((name) => String(process.env[name] || '').trim());
}

function fileExists(relativePath) {
    return existsSync(path.join(projectRoot, relativePath));
}

function gitIgnores(relativePath) {
    return run('git', ['check-ignore', '--no-index', relativePath]).status === 0;
}

function currentTargetTriple() {
    const result = run('rustc', ['--print', 'host-tuple']);
    const target = String(result.stdout || '').trim();

    return target || 'aarch64-apple-darwin';
}

function developerIdIdentity() {
    const configured = String(process.env.APPLE_SIGNING_IDENTITY || '').trim();

    if (configured) {
        return configured;
    }

    const result = run('security', ['find-identity', '-p', 'codesigning', '-v']);
    const output = `${result.stdout || ''}\n${result.stderr || ''}`;
    const match = output.match(/"((?:Developer ID Application:)[^"]+)"/);

    return match?.[1] || '';
}

function firstAppleApiKeyPath() {
    const directory = appleAuthDirectory();

    if (!existsSync(directory)) {
        return '';
    }

    const key = readdirSync(directory).find((entry) => /^AuthKey_[A-Z0-9]+\.p8$/.test(entry));

    return key ? path.join(directory, key) : '';
}

for (const command of ['node', 'npm', 'cargo', 'rustc', 'gh', 'stripe']) {
    record(commandExists(command) ? 'ok' : 'warn', `${command} CLI`, commandExists(command) ? 'available' : 'not found');
}

const wrangler = run(npxCommand, ['--yes', 'wrangler@latest', '--version']);
record(wrangler.status === 0 ? 'ok' : 'warn', 'wrangler CLI', wrangler.status === 0 ? String(wrangler.stdout || '').trim() : 'not available through npx');

const targetTriple = currentTargetTriple();
const mediaCheck = run(nodeCommand, [path.join(scriptDirectory, 'prepare-media-sidecars.mjs'), '--check']);
record(
    mediaCheck.status === 0 ? 'ok' : 'warn',
    `media sidecars (${targetTriple})`,
    mediaCheck.status === 0 ? 'ready' : String(mediaCheck.stderr || mediaCheck.stdout || '').trim()
);

record(
    fileExists('resources/fonts/gambado-sans-regular.otf') && fileExists('resources/fonts/gambado-sans-forte.otf') ? 'ok' : 'warn',
    'Gambado font files',
    'resources/fonts'
);
record(fileExists('resources/desktop/src/assets/dust-wave-square.png') ? 'ok' : 'warn', 'Dust Wave square logo', 'desktop asset');
record(fileExists('.github/workflows/desktop.yml') ? 'ok' : 'warn', 'desktop CI workflow', '.github/workflows/desktop.yml');

for (const relativePath of [
    'resources/dist',
    'resources/desktop/dist',
    'src-tauri/gen',
    'src-tauri/tauri.macos-signing.generated.conf.json',
    'src-tauri/target',
    'src-tauri/tauri.updater.generated.conf.json',
    `src-tauri/binaries/ffmpeg-${targetTriple}`,
    `src-tauri/binaries/ffprobe-${targetTriple}`,
    'src-tauri/binaries/SIDECARS.local.json',
]) {
    record(
        gitIgnores(relativePath) ? 'ok' : 'warn',
        `ignored generated artifact ${relativePath}`,
        '.gitignore'
    );
}

const releaseRepo = String(process.env.DUSTWAVE_RELEASE_REPO || process.env.GITHUB_REPOSITORY || gitRemoteRepoSlug(projectRoot) || '').trim();
const updaterPublicKey = envAny(['TAURI_UPDATER_PUBLIC_KEY', 'DUSTWAVE_TAURI_UPDATER_PUBLIC_KEY']) || existsSync(authPath('tauri-updater-public-key.txt'));
const updaterPrivateKey = envAny(['TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PATH']) || existsSync(authPath('tauri-updater-private.key'));
record(releaseRepo ? 'ok' : 'warn', 'updater release repository', releaseRepo || 'set DUSTWAVE_RELEASE_REPO=owner/repo');
record(updaterPublicKey ? 'ok' : 'warn', 'updater public key', updaterPublicKey ? 'present' : 'run npm run desktop:updater:keys or set TAURI_UPDATER_PUBLIC_KEY');
record(updaterPrivateKey ? 'ok' : 'warn', 'updater private key', updaterPrivateKey ? 'present locally or in env' : 'run npm run desktop:updater:keys or set TAURI_SIGNING_PRIVATE_KEY in release environment only');

const appleCertificate = envAny(['APPLE_CERTIFICATE']);
const appleP12 = existsSync(authPath('developer-id-application.p12'));
const appleApiKeyPath = String(process.env.APPLE_API_KEY_PATH || '').trim() || firstAppleApiKeyPath();
const appleApiKey = envAny(['APPLE_API_KEY']) || Boolean(appleApiKeyIdFromPath(appleApiKeyPath));
const appleIssuer = envAny(['APPLE_API_ISSUER']) || existsSync(authPath('apple-api-issuer.txt')) || existsSync(authPath('app-store-connect-issuer.txt')) || existsSync(authPath('issuer.txt'));
const appleSigningIdentity = developerIdIdentity();
const applePassword = envAny(['APPLE_CERTIFICATE_PASSWORD']);
const appleIdNotary = envAny(['APPLE_ID', 'APPLE_PASSWORD', 'APPLE_TEAM_ID']);
const appleApiNotary = appleApiKey && Boolean(appleApiKeyPath);
record(existsSync(appleAuthDirectory()) ? 'ok' : 'warn', 'Apple Auth directory', redactPath(appleAuthDirectory()));
record(appleCertificate || appleP12 || appleSigningIdentity ? 'ok' : 'warn', 'macOS signing material', 'APPLE_CERTIFICATE, Apple Auth .p12, or installed identity');
record(appleSigningIdentity ? 'ok' : 'warn', 'macOS signing identity', appleSigningIdentity || 'APPLE_SIGNING_IDENTITY or installed Developer ID Application identity');
record(!appleCertificate || applePassword ? 'ok' : 'warn', 'certificate password', 'APPLE_CERTIFICATE_PASSWORD or Apple Auth password file');
record(appleIdNotary || appleApiNotary ? 'ok' : 'warn', 'notarization credentials', appleApiNotary ? 'App Store Connect API key available' : 'Apple ID or App Store Connect API credentials');
record(!appleApiNotary || appleIssuer ? 'ok' : 'warn', 'Apple API issuer', 'required for Team API keys; omit only for Individual API keys');

for (const [label, names] of [
    ['X/Twitter credentials', ['DUSTWAVE_TWITTER_CLIENT_ID', 'TWITTER_CLIENT_ID']],
    ['Facebook credentials', ['DUSTWAVE_FACEBOOK_CLIENT_ID', 'FACEBOOK_CLIENT_ID']],
    ['Unsplash credentials', ['DUSTWAVE_UNSPLASH_CLIENT_ID', 'UNSPLASH_CLIENT_ID']],
    ['Klipy credentials', ['DUSTWAVE_KLIPY_CLIENT_ID', 'DUSTWAVE_KLIPY_API_KEY', 'KLIPY_CLIENT_ID', 'KLIPY_API_KEY']],
]) {
    record(envAny(names) ? 'ok' : 'warn', label, envAny(names) ? 'present in environment or keychain check still needed' : `set one of: ${names.join(', ')}`);
}

const remote = run('git', ['remote', 'get-url', 'origin']);
const remoteValue = String(remote.stdout || '').trim();
record(
    remoteValue.includes('dust') || remoteValue.includes('social') ? 'ok' : 'warn',
    'release git remote',
    remoteValue || 'no origin remote'
);

let warnings = 0;
for (const check of checks) {
    if (check.status !== 'ok') {
        warnings += 1;
    }

    const prefix = check.status === 'ok' ? '[ok]' : '[warn]';
    console.log(`${prefix} ${check.label}${check.detail ? ` - ${check.detail}` : ''}`);
}

if (warnings) {
    console.log(`\nPreflight completed with ${warnings} warning(s).`);
    if (strict) {
        process.exit(1);
    }
} else {
    console.log('\nPreflight completed without warnings.');
}
