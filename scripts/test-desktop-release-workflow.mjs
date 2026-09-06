#!/usr/bin/env node

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.resolve(scriptDirectory, '..');
const workflow = await readFile(
    path.join(projectRoot, '.github', 'workflows', 'desktop.yml'),
    'utf8',
);

const releaseCheck = workflow.match(/  release-check:\n([\s\S]*?)\n  release-bundle:/)?.[1] || '';
const releaseBundle = workflow.match(/  release-bundle:\n([\s\S]*)/)?.[1] || '';

assert.match(releaseCheck, /github\.ref_type != 'tag'/);
assert.match(releaseCheck, /Warm exact-commit Rust release compilation/);
assert.match(releaseCheck, /github\.ref == 'refs\/heads\/main'/);
assert.doesNotMatch(releaseBundle, /needs:\s*release-check/);
assert.match(releaseBundle, /Run desktop release checks[\s\S]*npm run desktop:release:check/);
assert.match(releaseBundle, /Restore exact-commit Rust release compilation/);
assert.match(workflow, /DUSTWAVE_RELEASE_USE_PROJECT_TARGET:\s*"true"/);

const rustCacheSteps = [...workflow.matchAll(
    /      - name: Restore (?:exact-commit )?Rust release compilation(?: cache)?\n([\s\S]*?)(?=\n      - name:)/g,
)].map((match) => match[1]);
assert.equal(rustCacheSteps.length, 2);
for (const cacheStep of rustCacheSteps) {
    assert.doesNotMatch(cacheStep, /restore-keys:/);
}
const cachedPaths = rustCacheSteps.flatMap((cacheStep) =>
    (cacheStep.match(/path: \|\n((?:\s{12}src-tauri\/target\/release\/[^\n]+\n)+)/)?.[1] || '')
        .trim()
        .split('\n')
        .filter(Boolean)
        .map((line) => line.trim()),
);
assert.deepEqual(
    [...new Set(cachedPaths)].sort(),
    [
        'src-tauri/target/release/.fingerprint',
        'src-tauri/target/release/build',
        'src-tauri/target/release/deps',
    ],
);
assert.equal(cachedPaths.some((cachedPath) => cachedPath.includes('/bundle')), false);
assert.equal(cachedPaths.some((cachedPath) => cachedPath.endsWith('/dust-wave-social')), false);
assert.match(releaseBundle, /npm run local-ai:native:offline -- --app/);
assert.ok(releaseBundle.indexOf('Smoke packaged launch and native offline inference') < releaseBundle.indexOf('Publish GitHub release'));
assert.match(releaseBundle, /node scripts\/release-notes\.mjs/);
assert.equal((releaseBundle.match(/--notes-file/g) || []).length, 2);
assert.doesNotMatch(releaseBundle, /--generate-notes/);

console.log('Desktop release workflow reuse tests passed.');
