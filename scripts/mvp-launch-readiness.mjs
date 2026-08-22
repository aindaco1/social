#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { gitRemoteRepoSlug } from './release-repo.js';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const strict = process.argv.includes('--strict');
const checks = [];

function run(command, args, options = {}) {
    return spawnSync(command, args, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
        env: process.env,
        ...options,
    });
}

function record(status, label, detail = '') {
    checks.push({ status, label, detail });
}

function commandExists(command) {
    const lookup = process.platform === 'win32' ? 'where' : 'which';

    return run(lookup, [command]).status === 0;
}

function envValue(...names) {
    for (const name of names) {
        const value = String(process.env[name] || '').trim();

        if (value) {
            return value;
        }
    }

    return '';
}

function ghNames(kind, repo) {
    if (!commandExists('gh') || !repo) {
        return new Set();
    }

    const result = run('gh', [kind, 'list', '--repo', repo]);

    if (result.status !== 0) {
        return new Set();
    }

    return new Set(
        String(result.stdout || '')
            .split(/\r?\n/)
            .map((line) => line.split(/\s+/)[0])
            .filter(Boolean),
    );
}

function ghVariable(name, repo) {
    if (!commandExists('gh') || !repo) {
        return '';
    }

    const result = run('gh', ['variable', 'get', name, '--repo', repo]);

    return result.status === 0 ? String(result.stdout || '').trim() : '';
}

function currentNotarizationId() {
    const explicit = envValue('DUSTWAVE_NOTARIZATION_SUBMISSION_ID', 'APPLE_NOTARIZATION_SUBMISSION_ID');

    if (explicit) {
        return explicit;
    }

    const planPath = path.join(projectRoot, 'docs', 'MVP_LAUNCH_PLAN.md');

    if (!existsSync(planPath)) {
        return '';
    }

    const plan = readFileSync(planPath, 'utf8');
    const match = plan.match(/submission `([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})`/i);

    return match?.[1] || '';
}

function documentedNotarizationAccepted(submissionId) {
    if (!submissionId) {
        return false;
    }

    const sources = [
        fileText('docs/MVP_LAUNCH_PLAN.md'),
    ];
    const idPattern = submissionId.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

    return sources.some((source) => {
        const nearSubmission = new RegExp(`(?:accepted|stapled)[\\s\\S]{0,160}${idPattern}|${idPattern}[\\s\\S]{0,160}(?:accepted|stapled)`, 'i');

        return nearSubmission.test(source);
    });
}

function fileText(relativePath) {
    const fullPath = path.join(projectRoot, relativePath);

    return existsSync(fullPath) ? readFileSync(fullPath, 'utf8') : '';
}

function firstClassInstagramProviderReady() {
    const providerSource = fileText('src-tauri/src/domain/provider.rs');
    const databaseSource = fileText('src-tauri/src/db/mod.rs');
    const desktopSource = fileText('resources/desktop/src/App.vue');

    return /id:\s*"instagram"/.test(providerSource)
        && /(?:refresh|import|publish)_instagram/.test(databaseSource)
        && /(?:connectInstagram|account\.provider === 'instagram'|'instagram')/.test(desktopSource);
}

function packageHasDependency(name) {
    const packageSource = fileText('package.json');

    if (!packageSource) {
        return false;
    }

    try {
        const packageJson = JSON.parse(packageSource);
        return Boolean(packageJson.dependencies?.[name] || packageJson.devDependencies?.[name]);
    } catch {
        return false;
    }
}

function localAiMediaCodeReady() {
    const desktopSource = fileText('resources/desktop/src/App.vue');
    const databaseSource = fileText('src-tauri/src/db/mod.rs');
    const noticesSource = fileText('THIRD_PARTY_NOTICES.md');

    return packageHasDependency('@litertjs/core')
        && /(?:LiteRT|local AI|localAi|ai media)/i.test(desktopSource)
        && /(?:upscal|preflight|smart crop|semantic|alt-text|alt_text)/i.test(desktopSource)
        && /save_local_ai_model_upscale_derivative/.test(desktopSource)
        && /(?:derivative|source_media|embedding|alt_text|ai_media)/i.test(databaseSource)
        && /file_hashes/.test(databaseSource)
        && /(?:LiteRT|model weights|tflite|local AI)/i.test(noticesSource);
}

function mediaStagingScheduledCleanupReady() {
    const workerSource = fileText('workers/media-staging/src/index.js');
    const templateConfig = fileText('workers/media-staging/wrangler.toml');
    const configGenerator = fileText('scripts/prepare-media-staging-config.mjs');
    const workflow = fileText('.github/workflows/media-staging.yml');

    return /async\s+scheduled\s*\(/.test(workerSource)
        && /ctx\.waitUntil/.test(workerSource)
        && /cleanupExpired\(env\)/.test(workerSource)
        && /crons\s*=\s*\[/.test(templateConfig)
        && /triggers:\s*\{[\s\S]*crons:\s*cronSchedules/.test(configGenerator)
        && /MEDIA_STAGING_CLEANUP_CRON/.test(workflow);
}

function localAiModelWeightsReady() {
    const result = run(process.execPath, [
        path.join(scriptDirectory, 'verify-local-ai-models.mjs'),
    ]);

    return result.status === 0;
}

function releaseNotesAndRollbackReady() {
    const scriptSource = fileText('scripts/generate-mvp-release-notes.mjs');
    const notesSource = fileText('docs/MVP_LAUNCH_PLAN.md');
    const submissionId = currentNotarizationId();

    return /Rollback Plan/.test(scriptSource)
        && /## Current Local Release Artifacts/.test(notesSource)
        && /## Rollback Plan/.test(notesSource)
        && /MVP readiness:\s+\d+\s+ready,\s+\d+\s+blocked,\s+\d+\s+manual/.test(notesSource)
        && /Updater URL:/.test(notesSource)
        && (!submissionId || notesSource.includes(submissionId));
}

function localReleaseArtifactSetReady() {
    const tauriSource = fileText('src-tauri/tauri.conf.json');

    if (!tauriSource) {
        return false;
    }

    try {
        const tauri = JSON.parse(tauriSource);
        const productName = tauri.productName || 'Dust Wave Social';
        const version = tauri.version || '0.1.0';
        const bundle = path.join(projectRoot, 'src-tauri', 'target', 'release', 'bundle');
        const files = [
            path.join(bundle, 'dmg', `${productName}_${version}_aarch64.dmg`),
            path.join(bundle, 'latest.json'),
            path.join(bundle, 'macos', `${productName}.app.tar.gz`),
            path.join(bundle, 'macos', `${productName}.app.tar.gz.sig`),
        ];

        return files.every((filePath) => existsSync(filePath));
    } catch {
        return false;
    }
}

function currentReleaseDmgPath() {
    const tauriSource = fileText('src-tauri/tauri.conf.json');

    if (!tauriSource) {
        return '';
    }

    try {
        const tauri = JSON.parse(tauriSource);
        const productName = tauri.productName || 'Dust Wave Social';
        const version = tauri.version || '0.1.0';

        return path.join(
            projectRoot,
            'src-tauri',
            'target',
            'release',
            'bundle',
            'dmg',
            `${productName}_${version}_aarch64.dmg`,
        );
    } catch {
        return '';
    }
}

function currentReleaseCandidateStapled() {
    const dmgPath = currentReleaseDmgPath();

    if (!dmgPath || !existsSync(dmgPath) || process.platform !== 'darwin') {
        return false;
    }

    return run('/usr/bin/xcrun', ['stapler', 'validate', dmgPath]).status === 0;
}

function notarizationStatus(submissionId) {
    if (!submissionId || process.platform !== 'darwin') {
        return '';
    }

    const result = run(process.execPath, [
        path.join(scriptDirectory, 'notarize-macos-app.mjs'),
        '--status',
        '--submission-id',
        submissionId,
    ]);
    const jsonStart = String(result.stdout || '').indexOf('{');

    if (result.status !== 0 || jsonStart < 0) {
        return '';
    }

    try {
        return JSON.parse(result.stdout.slice(jsonStart)).status || '';
    } catch {
        return '';
    }
}

function curlOk(url, expected = '') {
    const result = run('curl', ['-fsS', url]);
    const output = String(result.stdout || '');

    return result.status === 0 && (!expected || output.includes(expected));
}

const repo = envValue('DUSTWAVE_RELEASE_REPO', 'GITHUB_REPOSITORY') || gitRemoteRepoSlug(projectRoot) || '';
const secrets = ghNames('secret', repo);
const variables = ghNames('variable', repo);
const hasSecret = (name) => secrets.has(name) || Boolean(envValue(name));
const hasVariable = (name) => variables.has(name) || Boolean(envValue(name));
const brokerUrl = envValue('PUBLIC_BROKER_BASE_URL', 'TIKTOK_BROKER_PUBLIC_BASE_URL')
    || ghVariable('PUBLIC_BROKER_BASE_URL', repo);
const brokerHealthUrl = brokerUrl ? new URL('/api/health', brokerUrl.endsWith('/') ? brokerUrl : `${brokerUrl}/`).toString() : '';
const mediaStagingUrl = envValue('PUBLIC_MEDIA_BASE_URL', 'MEDIA_STAGING_PUBLIC_BASE_URL')
    || ghVariable('PUBLIC_MEDIA_BASE_URL', repo)
    || ghVariable('MEDIA_STAGING_PUBLIC_BASE_URL', repo);
const mediaStagingHealthUrl = mediaStagingUrl ? new URL('/api/health', mediaStagingUrl.endsWith('/') ? mediaStagingUrl : `${mediaStagingUrl}/`).toString() : '';

record(repo ? 'ready' : 'blocked', 'GitHub release repository', repo || 'set DUSTWAVE_RELEASE_REPO');
record(hasSecret('APPLE_CERTIFICATE') ? 'ready' : 'blocked', 'Apple signing certificate secret', 'APPLE_CERTIFICATE');
record(hasSecret('APPLE_API_KEY_P8') && hasSecret('APPLE_API_KEY') ? 'ready' : 'blocked', 'Apple notarization API key secrets', 'APPLE_API_KEY and APPLE_API_KEY_P8');
record(hasSecret('TAURI_SIGNING_PRIVATE_KEY') && hasVariable('TAURI_UPDATER_PUBLIC_KEY') ? 'ready' : 'blocked', 'Tauri updater signing settings', 'private key secret and public key variable');
record(brokerUrl ? 'ready' : 'blocked', 'TikTok broker base URL', brokerUrl || 'set PUBLIC_BROKER_BASE_URL');
record(
    brokerHealthUrl && curlOk(brokerHealthUrl, '"ok": true') ? 'ready' : 'blocked',
    'TikTok broker health endpoint',
    brokerHealthUrl || 'broker URL missing',
);

for (const name of [
    'TIKTOK_BROKER_D1_DATABASE_ID',
    'TOKEN_ENCRYPTION_KEY',
    'BROKER_ADMIN_TOKEN',
]) {
    record(hasSecret(name) ? 'ready' : 'blocked', `TikTok broker secret ${name}`, 'GitHub secret or local env');
}

for (const name of ['TIKTOK_CLIENT_KEY', 'TIKTOK_CLIENT_SECRET']) {
    record(hasSecret(name) ? 'ready' : 'manual', `TikTok developer credential ${name}`, 'from TikTok Developer Portal');
}

record(hasSecret('CLOUDFLARE_API_TOKEN') ? 'ready' : 'manual', 'Cloudflare CI deploy token', 'must include Workers and D1 permissions');
record(hasSecret('CLOUDFLARE_ACCOUNT_ID') ? 'ready' : 'blocked', 'Cloudflare account ID', 'GitHub secret or local env');

record(
    firstClassInstagramProviderReady() ? 'ready' : 'blocked',
    'Instagram first-class provider implementation',
    'account connection, publish/schedule, import, reporting, and UI acceptance',
);

record(hasSecret('MEDIA_STAGING_TOKEN') ? 'ready' : 'blocked', 'Media staging token', 'GitHub secret or local env');
record(hasVariable('MEDIA_STAGING_BUCKET_NAME') ? 'ready' : 'manual', 'Media staging R2 bucket name', 'GitHub variable or local env');
record(mediaStagingUrl ? 'ready' : 'blocked', 'Media staging public base URL', mediaStagingUrl || 'set PUBLIC_MEDIA_BASE_URL');
record(
    mediaStagingHealthUrl && curlOk(mediaStagingHealthUrl, '"ok":true') ? 'ready' : 'blocked',
    'Media staging health endpoint',
    mediaStagingHealthUrl || 'media staging URL missing',
);

record(
    mediaStagingScheduledCleanupReady() ? 'ready' : 'blocked',
    'Media staging scheduled cleanup implementation',
    'Worker scheduled handler and Wrangler cron config',
);

record(
    localAiMediaCodeReady() ? 'ready' : 'blocked',
    'Local AI media code implementation',
    'LiteRT.js bundled runtime, Labs flag, model-backed upscaling derivatives, preflight, crops, profile search, review-required alt-text drafts',
);

record(
    localAiModelWeightsReady() ? 'ready' : 'manual',
    'Local AI model bundle and checksum validation',
    'licensed model manifest, model files, metadata, and notice checks',
);

record(
    releaseNotesAndRollbackReady() ? 'ready' : 'manual',
    'Release notes and rollback draft',
    'run npm run mvp:release:notes after rebuilding release artifacts',
);

record(
    localReleaseArtifactSetReady() ? 'ready' : 'blocked',
    'Local release artifact set',
    'DMG, latest.json, updater archive, and updater signature under src-tauri/target/release/bundle',
);

record(
    currentReleaseCandidateStapled() ? 'ready' : 'manual',
    'Current release candidate notarization and stapling',
    currentReleaseDmgPath()
        ? 'submit the current DMG to Apple, wait for acceptance, staple it, and rerun strict artifact verification'
        : 'build the local release artifact set first',
);

record(
    'manual',
    'Instagram Local Media paired in Connections > Provider setup and local-image acceptance',
    'requires launch Mac Keychain entry and live Instagram publish validation',
);

record(
    'manual',
    'Local AI packaged-app offline model probe and reviewed output acceptance',
    'requires signed/stapled app test with network disabled and operator review of generated derivatives',
);

for (const label of [
    'X/Twitter live credential and publish acceptance',
    'Facebook/Meta live credential and Page acceptance',
    'Instagram live credential, publishing, scheduling, and insights acceptance',
    'Unsplash live credential acceptance',
    'Klipy production key and attribution acceptance',
    'Dust Wave account onboarding and live publish/import acceptance',
    'Clean-Mac Gatekeeper install test',
]) {
    record('manual', label, 'requires provider portal, live account, or separate target Mac');
}

record(
    'manual',
    'Operator updater installation, relaunch, and app-data acceptance',
    'requires the installed previous public release and representative app data on the target Mac',
);

const submissionId = currentNotarizationId();
const status = notarizationStatus(submissionId);
const documentedAccepted = documentedNotarizationAccepted(submissionId);

record(
    status === 'Accepted' || documentedAccepted ? 'ready' : 'manual',
    'Historical Apple notarization acceptance',
    status
        ? `${submissionId}: ${status}`
        : documentedAccepted
            ? `${submissionId}: accepted/stapled in launch docs`
            : 'not checked or still pending',
);

const counts = { ready: 0, blocked: 0, manual: 0 };

for (const check of checks) {
    counts[check.status] += 1;
    const prefix = check.status === 'ready'
        ? '[ready]'
        : check.status === 'blocked'
            ? '[blocked]'
            : '[manual]';

    console.log(`${prefix} ${check.label}${check.detail ? ` - ${check.detail}` : ''}`);
}

console.log(`\nMVP readiness: ${counts.ready} ready, ${counts.blocked} blocked, ${counts.manual} manual.`);

if (strict && (counts.blocked > 0 || counts.manual > 0)) {
    process.exit(1);
}
