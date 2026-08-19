#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
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
const commandAvailability = new Map();

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
    if (commandAvailability.has(command)) {
        return commandAvailability.get(command);
    }

    const lookup = process.platform === 'win32' ? 'where' : 'which';
    const exists = run(lookup, [command]).status === 0;

    commandAvailability.set(command, exists);

    return exists;
}

function envAny(names) {
    return names.some((name) => String(process.env[name] || '').trim());
}

function envValue(names) {
    for (const name of names) {
        const value = String(process.env[name] || '').trim();

        if (value) {
            return value;
        }
    }

    return '';
}

function fileExists(relativePath) {
    return existsSync(path.join(projectRoot, relativePath));
}

function fileText(relativePath) {
    const fullPath = path.join(projectRoot, relativePath);

    return existsSync(fullPath) ? readFileSync(fullPath, 'utf8') : '';
}

function mediaStagingScheduledCleanupReady() {
    const workerSource = fileText('workers/media-staging/src/index.js');
    const templateConfig = fileText('workers/media-staging/wrangler.toml');
    const configGenerator = fileText('scripts/prepare-media-staging-config.mjs');

    return /async\s+scheduled\s*\(/.test(workerSource)
        && /ctx\.waitUntil/.test(workerSource)
        && /cleanupExpired\(env\)/.test(workerSource)
        && /crons\s*=\s*\[/.test(templateConfig)
        && /triggers:\s*\{[\s\S]*crons:\s*cronSchedules/.test(configGenerator);
}

function releaseNotesGeneratorReady() {
    const generatorSource = fileText('scripts/generate-mvp-release-notes.mjs');
    const notesSource = fileText('docs/MVP_LAUNCH_PLAN.md');

    return /MVP_RELEASE_NOTES_START/.test(generatorSource)
        && /Rollback Plan/.test(generatorSource)
        && /## Current Local Release Artifacts/.test(notesSource)
        && /## Rollback Plan/.test(notesSource);
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

function ghSettingNames(kind, repo) {
    if (!commandExists('gh') || !repo) {
        return { names: new Set(), error: 'gh CLI or release repository missing' };
    }

    const result = run('gh', [kind, 'list', '--repo', repo]);

    if (result.status !== 0) {
        return {
            names: new Set(),
            error: String(result.stderr || result.stdout || '').trim() || `gh ${kind} list failed`,
        };
    }

    const names = new Set(
        String(result.stdout || '')
            .split(/\r?\n/)
            .map((line) => line.split(/\s+/)[0])
            .filter(Boolean),
    );

    return { names, error: '' };
}

function ghVariableValue(name, repo) {
    if (!commandExists('gh') || !repo) {
        return '';
    }

    const result = run('gh', ['variable', 'get', name, '--repo', repo]);

    return result.status === 0 ? String(result.stdout || '').trim() : '';
}

function hasOkJson(output) {
    return /"ok"\s*:\s*true/.test(String(output || ''));
}

for (const command of ['node', 'npm', 'cargo', 'rustc', 'gh', 'stripe']) {
    const exists = commandExists(command);

    record(exists ? 'ok' : 'warn', `${command} CLI`, exists ? 'available' : 'not found');
}

const wrangler = run(npxCommand, ['wrangler', '--version']);
record(wrangler.status === 0 ? 'ok' : 'warn', 'wrangler CLI', wrangler.status === 0 ? String(wrangler.stdout || '').trim() : 'not available through npx');

const targetTriple = currentTargetTriple();
const mediaCheck = run(nodeCommand, [path.join(scriptDirectory, 'prepare-media-sidecars.mjs'), '--check']);
record(
    mediaCheck.status === 0 ? 'ok' : 'warn',
    `media sidecars (${targetTriple})`,
    mediaCheck.status === 0 ? 'ready' : String(mediaCheck.stderr || mediaCheck.stdout || '').trim()
);
const localAiModelCheck = run(nodeCommand, [path.join(scriptDirectory, 'verify-local-ai-models.mjs')]);
record(
    localAiModelCheck.status === 0 ? 'ok' : 'warn',
    'local AI model bundle',
    localAiModelCheck.status === 0 ? String(localAiModelCheck.stdout || '').trim() : String(localAiModelCheck.stderr || localAiModelCheck.stdout || '').trim()
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
    'workers/tiktok-broker/wrangler.generated.jsonc',
    'workers/tiktok-broker/.dev.vars',
    'workers/tiktok-broker/.wrangler',
    'workers/media-staging/wrangler.generated.jsonc',
    'workers/media-staging/.dev.vars',
    'workers/media-staging/.wrangler',
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
const repoSecrets = ghSettingNames('secret', releaseRepo);
const repoVariables = ghSettingNames('variable', releaseRepo);
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

if (releaseRepo && !repoSecrets.error) {
    record('ok', 'GitHub secret name lookup', `${releaseRepo} available`);
} else {
    record('warn', 'GitHub secret name lookup', repoSecrets.error || 'set DUSTWAVE_RELEASE_REPO=owner/repo');
}

if (releaseRepo && !repoVariables.error) {
    record('ok', 'GitHub variable name lookup', `${releaseRepo} available`);
} else {
    record('warn', 'GitHub variable name lookup', repoVariables.error || 'set DUSTWAVE_RELEASE_REPO=owner/repo');
}

for (const [label, names] of [
    ['X/Twitter credentials', ['DUSTWAVE_TWITTER_CLIENT_ID', 'TWITTER_CLIENT_ID']],
    ['Facebook credentials', ['DUSTWAVE_FACEBOOK_CLIENT_ID', 'FACEBOOK_CLIENT_ID']],
    ['Unsplash credentials', ['DUSTWAVE_UNSPLASH_CLIENT_ID', 'UNSPLASH_CLIENT_ID']],
    ['Klipy credentials', ['DUSTWAVE_KLIPY_CLIENT_ID', 'DUSTWAVE_KLIPY_API_KEY', 'KLIPY_CLIENT_ID', 'KLIPY_API_KEY']],
    ['TikTok desktop Client Key', ['DUSTWAVE_TIKTOK_CLIENT_KEY', 'TIKTOK_CLIENT_KEY']],
]) {
    record(envAny(names) ? 'ok' : 'warn', label, envAny(names) ? 'present in environment or keychain check still needed' : `set one of: ${names.join(', ')}`);
}

record(fileExists('workers/tiktok-broker/src/index.js') ? 'ok' : 'warn', 'TikTok broker Worker', 'workers/tiktok-broker');
record(fileExists('.github/workflows/tiktok-broker.yml') ? 'ok' : 'warn', 'TikTok broker CI workflow', '.github/workflows/tiktok-broker.yml');
record(fileExists('scripts/prepare-tiktok-broker-config.mjs') ? 'ok' : 'warn', 'TikTok broker config generator', 'scripts/prepare-tiktok-broker-config.mjs');
record(fileExists('scripts/configure-github-tiktok-broker-secrets.mjs') ? 'ok' : 'warn', 'TikTok broker GitHub settings helper', 'scripts/configure-github-tiktok-broker-secrets.mjs');
record(fileExists('workers/media-staging/src/index.js') ? 'ok' : 'warn', 'Media staging Worker', 'workers/media-staging');
record(fileExists('.github/workflows/media-staging.yml') ? 'ok' : 'warn', 'Media staging CI workflow', '.github/workflows/media-staging.yml');
record(fileExists('scripts/prepare-media-staging-config.mjs') ? 'ok' : 'warn', 'Media staging config generator', 'scripts/prepare-media-staging-config.mjs');
record(fileExists('scripts/configure-github-media-staging-secrets.mjs') ? 'ok' : 'warn', 'Media staging GitHub settings helper', 'scripts/configure-github-media-staging-secrets.mjs');
record(
    mediaStagingScheduledCleanupReady() ? 'ok' : 'warn',
    'Media staging scheduled cleanup',
    'Worker scheduled handler and Wrangler cron config'
);
record(
    releaseNotesGeneratorReady() ? 'ok' : 'warn',
    'MVP release notes and rollback draft',
    'docs/MVP_LAUNCH_PLAN.md'
);

const hasRepoSecret = (name) => repoSecrets.names.has(name);
const hasRepoVariable = (name) => repoVariables.names.has(name);
const missingCloudflareSettings = ['CLOUDFLARE_API_TOKEN', 'CLOUDFLARE_ACCOUNT_ID'].filter(
    (name) => !envAny([name]) && !hasRepoSecret(name),
);
const missingBrokerSettings = [
    'TIKTOK_BROKER_D1_DATABASE_ID',
    'TIKTOK_CLIENT_KEY',
    'TIKTOK_CLIENT_SECRET',
    'TOKEN_ENCRYPTION_KEY',
    'BROKER_ADMIN_TOKEN',
].filter((name) => !envAny([name]) && !hasRepoSecret(name));
const brokerBaseUrl = envValue(['PUBLIC_BROKER_BASE_URL', 'TIKTOK_BROKER_PUBLIC_BASE_URL'])
    || ghVariableValue('PUBLIC_BROKER_BASE_URL', releaseRepo);
const brokerBaseUrlConfigured = Boolean(brokerBaseUrl) || hasRepoVariable('PUBLIC_BROKER_BASE_URL');
const missingMediaStagingSecrets = [
    'MEDIA_STAGING_TOKEN',
].filter((name) => !envAny([name]) && !hasRepoSecret(name));
const missingMediaStagingVariables = [
    'MEDIA_STAGING_WORKER_NAME',
    'MEDIA_STAGING_BUCKET_NAME',
    'PUBLIC_MEDIA_BASE_URL',
].filter((name) => !envAny([name]) && !hasRepoVariable(name));
const mediaStagingBaseUrl = envValue(['PUBLIC_MEDIA_BASE_URL', 'MEDIA_STAGING_PUBLIC_BASE_URL'])
    || ghVariableValue('PUBLIC_MEDIA_BASE_URL', releaseRepo);
const mediaStagingBaseUrlConfigured = Boolean(mediaStagingBaseUrl) || hasRepoVariable('PUBLIC_MEDIA_BASE_URL');

record(
    missingCloudflareSettings.length === 0 ? 'ok' : 'warn',
    'TikTok broker Cloudflare deployment settings',
    missingCloudflareSettings.length === 0 ? 'present in env or GitHub secrets' : `missing: ${missingCloudflareSettings.join(', ')}`
);
record(
    missingBrokerSettings.length === 0 ? 'ok' : 'warn',
    'TikTok broker D1 and Worker secrets',
    missingBrokerSettings.length === 0 ? 'present in env or GitHub secrets' : `missing: ${missingBrokerSettings.join(', ')}`
);
record(
    brokerBaseUrlConfigured ? 'ok' : 'warn',
    'TikTok broker public base URL',
    brokerBaseUrl ? brokerBaseUrl : 'set PUBLIC_BROKER_BASE_URL after deployment'
);

if (brokerBaseUrl) {
    const healthUrl = new URL('/api/health', brokerBaseUrl.endsWith('/') ? brokerBaseUrl : `${brokerBaseUrl}/`).toString();
    const brokerHealth = run('curl', ['-fsS', healthUrl]);

    record(
        brokerHealth.status === 0 && hasOkJson(brokerHealth.stdout) ? 'ok' : 'warn',
        'TikTok broker health endpoint',
        brokerHealth.status === 0 ? healthUrl : String(brokerHealth.stderr || brokerHealth.stdout || '').trim()
    );
}

record(
    missingCloudflareSettings.length === 0 ? 'ok' : 'warn',
    'Media staging Cloudflare deployment settings',
    missingCloudflareSettings.length === 0 ? 'present in env or GitHub secrets' : `missing: ${missingCloudflareSettings.join(', ')}`
);
record(
    missingMediaStagingSecrets.length === 0 ? 'ok' : 'warn',
    'Media staging Worker secrets',
    missingMediaStagingSecrets.length === 0 ? 'present in env or GitHub secrets' : `missing: ${missingMediaStagingSecrets.join(', ')}`
);
record(
    missingMediaStagingVariables.length === 0 ? 'ok' : 'warn',
    'Media staging Worker variables',
    missingMediaStagingVariables.length === 0 ? 'present in env or GitHub variables' : `missing: ${missingMediaStagingVariables.join(', ')}`
);
record(
    mediaStagingBaseUrlConfigured ? 'ok' : 'warn',
    'Media staging public base URL',
    mediaStagingBaseUrl || 'set PUBLIC_MEDIA_BASE_URL after deployment'
);

if (mediaStagingBaseUrl) {
    const healthUrl = new URL('/api/health', mediaStagingBaseUrl.endsWith('/') ? mediaStagingBaseUrl : `${mediaStagingBaseUrl}/`).toString();
    const mediaStagingHealth = run('curl', ['-fsS', healthUrl]);

    record(
        mediaStagingHealth.status === 0 && hasOkJson(mediaStagingHealth.stdout) ? 'ok' : 'warn',
        'Media staging health endpoint',
        mediaStagingHealth.status === 0 ? healthUrl : String(mediaStagingHealth.stderr || mediaStagingHealth.stdout || '').trim()
    );
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
