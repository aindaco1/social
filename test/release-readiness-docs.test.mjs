import test from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, mkdirSync, readdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { LOCAL_RELEASE_READINESS_PATH, RELEASE_OPERATIONS_PATH, releaseReadinessDocumentationReady } from '../scripts/lib/release-readiness-docs.mjs';

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

test('local reports and durable release documentation have one shared contract', () => {
    assert.equal(LOCAL_RELEASE_READINESS_PATH, 'artifacts/release-readiness.md');
    assert.equal(RELEASE_OPERATIONS_PATH, 'docs/RELEASE_OPERATIONS.md');
    assert.equal(releaseReadinessDocumentationReady(projectRoot), true);
});

test('readiness refuses stale committed snapshots and missing rollback guidance', () => {
    const root = mkdtempSync(path.join(os.tmpdir(), 'social-readiness-docs-'));
    try {
        for (const name of ['docs', 'scripts']) mkdirSync(path.join(root, name));
        for (const name of ['scripts/generate-mvp-release-notes.mjs', RELEASE_OPERATIONS_PATH, '.gitignore']) {
            writeFileSync(path.join(root, name), readFileSync(path.join(projectRoot, name)));
        }
        assert.equal(releaseReadinessDocumentationReady(root), true);
        const runbook = readFileSync(path.join(root, RELEASE_OPERATIONS_PATH), 'utf8');
        writeFileSync(path.join(root, RELEASE_OPERATIONS_PATH), runbook + '\n<!-- MVP_RELEASE_NOTES_START -->');
        assert.equal(releaseReadinessDocumentationReady(root), false);
        writeFileSync(path.join(root, RELEASE_OPERATIONS_PATH), runbook.replace('## Rollback Plan', '## Removed'));
        assert.equal(releaseReadinessDocumentationReady(root), false);
        writeFileSync(path.join(root, RELEASE_OPERATIONS_PATH), runbook);
        writeFileSync(path.join(root, '.gitignore'), '');
        assert.equal(releaseReadinessDocumentationReady(root), false);
    } finally {
        rmSync(root, { recursive: true, force: true });
    }
});

test('an explicit snapshot output cannot overwrite maintained or tracked documentation', () => {
    for (const relativePath of [RELEASE_OPERATIONS_PATH, 'docs/CHANGELOG.md', 'docs/THIRD_PARTY_NOTICES.md', 'README.md']) {
        const before = readFileSync(path.join(projectRoot, relativePath));
        const result = spawnSync(process.execPath, ['scripts/generate-mvp-release-notes.mjs', '--output', relativePath], {
            cwd: projectRoot, encoding: 'utf8', timeout: 5000,
        });
        assert.equal(result.status, 1);
        assert.match(result.stderr, /Refusing to write.*tracked file/);
        assert.deepEqual(readFileSync(path.join(projectRoot, relativePath)), before);
    }
});

test('documentation moves preserve root entry points, release history, and packaged notices', () => {
    assert.deepEqual(readdirSync(projectRoot).filter(name => name.endsWith('.md')).sort(), ['LICENSE.md', 'README.md', 'SECURITY.md']);
    for (const name of ['docs/CHANGELOG.md', 'docs/THIRD_PARTY_NOTICES.md', RELEASE_OPERATIONS_PATH]) {
        assert.equal(existsSync(path.join(projectRoot, name)), true, name);
    }
    const config = JSON.parse(readFileSync(path.join(projectRoot, 'src-tauri/tauri.conf.json'), 'utf8'));
    const noticeSources = Object.entries(config.bundle.resources).filter(([, destination]) => destination === 'THIRD_PARTY_NOTICES.md');
    assert.deepEqual(noticeSources, [['../docs/THIRD_PARTY_NOTICES.md', 'THIRD_PARTY_NOTICES.md']]);
    const workflow = readFileSync(path.join(projectRoot, '.github/workflows/desktop.yml'), 'utf8');
    assert.match(workflow, /hashFiles\('scripts\/build-ffmpeg-lgpl-sidecars\.mjs', 'docs\/THIRD_PARTY_NOTICES\.md'\)/);
});
