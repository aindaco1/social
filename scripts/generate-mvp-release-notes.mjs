#!/usr/bin/env node

import { createHash } from 'node:crypto';
import {
    existsSync,
    mkdirSync,
    readFileSync,
    readdirSync,
    statSync,
    writeFileSync,
} from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { gitRemoteRepoSlug } from './release-repo.js';
import { LOCAL_RELEASE_READINESS_PATH, RELEASE_OPERATIONS_PATH } from './lib/release-readiness-docs.mjs';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const args = process.argv.slice(2);
const check = args.includes('--check');
const outputPath = path.resolve(projectRoot, argValue('--output') || LOCAL_RELEASE_READINESS_PATH);
const sectionStart = '<!-- MVP_RELEASE_NOTES_START -->';
const sectionEnd = '<!-- MVP_RELEASE_NOTES_END -->';

// Generated machine state is disposable. An explicit override may select another
// untracked report path, but must not replace durable source documentation—even
// newly created or relocated guides that have not been staged yet.
const relativeOutput = path.relative(projectRoot, outputPath);
const trackedOutput = run('git', ['ls-files', '--error-unmatch', '--', relativeOutput]);
if (trackedOutput.status === 0 || relativeOutput === 'docs' || relativeOutput.startsWith(`docs${path.sep}`)) {
    console.error('Refusing to write a local readiness snapshot into a tracked file or maintained documentation path. Use artifacts/release-readiness.md or another untracked output path outside docs/.');
    process.exit(1);
}

function argValue(name) {
    const index = args.indexOf(name);

    return index >= 0 ? args[index + 1] : '';
}

function relative(filePath) {
    return path.relative(projectRoot, filePath);
}

function run(command, commandArgs, options = {}) {
    return spawnSync(command, commandArgs, {
        cwd: projectRoot,
        encoding: 'utf8',
        shell: false,
        stdio: 'pipe',
        env: process.env,
        ...options,
    });
}

function readJson(relativePath) {
    return JSON.parse(readFileSync(path.join(projectRoot, relativePath), 'utf8'));
}

function readText(relativePath) {
    const filePath = path.join(projectRoot, relativePath);

    return existsSync(filePath) ? readFileSync(filePath, 'utf8') : '';
}

function sha256(filePath) {
    return createHash('sha256').update(readFileSync(filePath)).digest('hex');
}

function formatBytes(bytes) {
    if (bytes < 1024) {
        return `${bytes} B`;
    }

    const units = ['KB', 'MB', 'GB'];
    let value = bytes / 1024;
    let unit = units.shift();

    while (value >= 1024 && units.length > 0) {
        value /= 1024;
        unit = units.shift();
    }

    return `${value.toFixed(value >= 10 ? 0 : 1)} ${unit}`;
}

function artifactSummary(label, filePath, options = {}) {
    if (!existsSync(filePath)) {
        return `- ${label}: missing at \`${relative(filePath)}\``;
    }

    const checksum = options.checksum === false ? '' : `, SHA-256 \`${sha256(filePath)}\``;

    return `- ${label}: \`${relative(filePath)}\` (${formatBytes(statSync(filePath).size)}${checksum})`;
}

function releaseSourceState(version) {
    const explicit = String(process.env.DUSTWAVE_RELEASE_COMMIT || process.env.DUSTWAVE_RELEASE_TAG || '').trim();

    if (explicit) {
        return explicit;
    }

    const releaseTag = `v${version}`;
    const tag = run('git', ['rev-parse', '--verify', `refs/tags/${releaseTag}^{commit}`]);

    if (tag.status === 0) {
        return `release tag ${releaseTag} exists; the checkout may include post-release changes`;
    }

    const dirty = run('git', ['status', '--porcelain']);

    return dirty.status === 0 && String(dirty.stdout || '').trim()
        ? 'generated from local worktree with uncommitted changes'
        : 'pending final release tag';
}

function currentNotarizationId() {
    const explicit = String(
        process.env.DUSTWAVE_NOTARIZATION_SUBMISSION_ID ||
        process.env.APPLE_NOTARIZATION_SUBMISSION_ID ||
        '',
    ).trim();

    if (explicit) {
        return explicit;
    }

    const plan = readText(RELEASE_OPERATIONS_PATH);
    const match = plan.match(/Apple accepted (?:DMG )?submission `([0-9a-f-]{36})`/i);

    return match?.[1] || '';
}

function releaseDmgPath(productName, version) {
    const dmgDirectory = path.join(projectRoot, 'src-tauri', 'target', 'release', 'bundle', 'dmg');
    const expected = path.join(dmgDirectory, `${productName}_${version}_aarch64.dmg`);

    if (existsSync(expected) || !existsSync(dmgDirectory)) {
        return expected;
    }

    const match = readdirSync(dmgDirectory)
        .filter((entry) => entry.endsWith('.dmg') && entry.startsWith(`${productName}_${version}`))
        .sort()
        .at(-1);

    return match ? path.join(dmgDirectory, match) : expected;
}

function readinessLines() {
    const result = run(process.execPath, [path.join(scriptDirectory, 'mvp-launch-readiness.mjs')]);
    const output = `${result.stdout || ''}${result.stderr || ''}`.trim();
    const lines = output.split(/\r?\n/).filter(Boolean);
    const summary = lines.find((line) => line.startsWith('MVP readiness:')) || 'MVP readiness: not checked.';
    const blocked = lines.filter((line) => line.startsWith('[blocked]'));
    const manual = lines.filter((line) => line.startsWith('[manual]'));

    return {
        summary,
        blocked: blocked.length > 0 ? blocked : ['[blocked] No blocking items were reported by the readiness script.'],
        manual: manual.length > 0 ? manual : ['[manual] No manual items were reported by the readiness script.'],
    };
}

function latestJsonSummary(latestJsonPath) {
    if (!existsSync(latestJsonPath)) {
        return {
            version: '',
            url: '',
            signaturePresent: false,
        };
    }

    const latest = JSON.parse(readFileSync(latestJsonPath, 'utf8'));
    const platform = latest.platforms?.['darwin-aarch64'] || {};

    return {
        version: latest.version || '',
        url: platform.url || '',
        signaturePresent: Boolean(platform.signature),
    };
}

function renderNotes() {
    const tauri = readJson('src-tauri/tauri.conf.json');
    const productName = tauri.productName || 'Dust Wave Social';
    const version = tauri.version || '0.1.0';
    const repo = String(process.env.DUSTWAVE_RELEASE_REPO || process.env.GITHUB_REPOSITORY || gitRemoteRepoSlug(projectRoot) || '').trim() || 'aindaco1/social';
    const sourceState = releaseSourceState(version);
    const submissionId = currentNotarizationId();
    const dmgPath = releaseDmgPath(productName, version);
    const generatedAt = existsSync(dmgPath)
        ? statSync(dmgPath).mtime.toISOString()
        : 'not generated; no local DMG';
    const latestJsonPath = path.join(projectRoot, 'src-tauri', 'target', 'release', 'bundle', 'latest.json');
    const updaterArchivePath = path.join(projectRoot, 'src-tauri', 'target', 'release', 'bundle', 'macos', `${productName}.app.tar.gz`);
    const updaterSignaturePath = `${updaterArchivePath}.sig`;
    const latest = latestJsonSummary(latestJsonPath);
    const completeArtifactSet = [dmgPath, latestJsonPath, updaterArchivePath, updaterSignaturePath]
        .every((filePath) => existsSync(filePath));
    const releaseState = completeArtifactSet
        ? 'complete local Apple Silicon artifact set present; verify it independently before using it for any later publication.'
        : 'no complete local release candidate; recover or rebuild the missing artifacts before acceptance or publication.';
    const readiness = readinessLines();
    const blockedItems = readiness.blocked
        .map((line) => `- ${line.replace(/^\[blocked\]\s*/, '')}`)
        .join('\n');
    const manualItems = readiness.manual
        .map((line) => `- ${line.replace(/^\[manual]\s*/, '')}`)
        .join('\n');

    return `${sectionStart}
## Current Local Release Artifacts

Generated: ${generatedAt}

Repository: \`${repo}\`
Source state: ${sourceState}
Release state: ${releaseState}

## Artifacts

${artifactSummary('Apple Silicon DMG', dmgPath)}
- Recorded notarization submission (verify it matches this DMG): \`${submissionId || 'not recorded'}\`
${artifactSummary('Tauri updater latest.json', latestJsonPath, { checksum: false })}
${artifactSummary('Tauri updater archive', updaterArchivePath)}
${artifactSummary('Tauri updater signature', updaterSignaturePath, { checksum: false })}
- Updater version: \`${latest.version || version}\`
- Updater URL: ${latest.url ? `\`${latest.url}\`` : 'not generated'}
- Updater signature embedded in latest.json: ${latest.signaturePresent ? 'yes' : 'no'}

## Readiness Snapshot

${readiness.summary}

Blocking issues:

${blockedItems}

Manual acceptance still required:

${manualItems}

## Next checks

This report describes local checkout artifacts, not the availability or acceptance
of the public release. Missing artifacts after cleanup are expected.

Use the maintained release/build/updater/rollback procedure in
[Release operations](<${path.relative(path.dirname(outputPath), path.join(projectRoot, RELEASE_OPERATIONS_PATH)).split(path.sep).join('/')}>)
and the current priorities in
[Project status](<${path.relative(path.dirname(outputPath), path.join(projectRoot, 'docs/PROJECT_STATUS.md')).split(path.sep).join('/')}>).
Do not copy this machine snapshot into source documentation.
${sectionEnd}`;
}

const content = renderNotes();
const existing = existsSync(outputPath) ? readFileSync(outputPath, 'utf8') : '# Local release readiness\n';

function replaceSection(source, replacement) {
    const start = source.indexOf(sectionStart);
    const end = source.indexOf(sectionEnd);

    if (start >= 0 && end > start) {
        return `${source.slice(0, start)}${replacement}${source.slice(end + sectionEnd.length)}`;
    }

    return `${source.trimEnd()}\n\n${replacement}\n`;
}

if (check) {
    const start = existing.indexOf(sectionStart);
    const end = existing.indexOf(sectionEnd);
    const current = start >= 0 && end > start
        ? existing.slice(start, end + sectionEnd.length)
        : '';

    if (current !== content) {
        console.error(`${relative(outputPath)} is out of date. Run npm run mvp:release:notes.`);
        process.exit(1);
    }

    console.log(`${relative(outputPath)} is current.`);
} else {
    mkdirSync(path.dirname(outputPath), { recursive: true });
    writeFileSync(outputPath, replaceSection(existing, content));
    console.log(`Wrote ${relative(outputPath)}`);
}
