import test from 'node:test';
import assert from 'node:assert/strict';
import { extractReleaseNotes, releaseNotes } from '../scripts/release-notes.mjs';

test('release and updater notes come from only the requested changelog entry', () => {
    const changelog = '# Changelog\n\n## 0.1.10 - 2026-09-06\n\nNew release.\n\n### Testing\nStill open.\n\n## 0.1.9 - 2026-08-26\n\nOlder release.\n';
    assert.equal(extractReleaseNotes(changelog, '0.1.10'), 'New release.\n\n### Testing\nStill open.');
    assert.equal(extractReleaseNotes(changelog, '0.1.9'), 'Older release.');
    assert.throws(() => extractReleaseNotes(changelog, '0.1.8'), /Missing changelog/);
    assert.throws(() => extractReleaseNotes(changelog, '0.1.10-test'), /stable desktop version/);
});

test('0.1.10 notes retain opt-in instructions and the outstanding acceptance boundary', () => {
    const notes = releaseNotes('0.1.10');
    assert.match(notes, /Labs remains opt-in/);
    assert.match(notes, /Full-app offline isolation/);
    assert.match(notes, /human\noutput-quality approval remain unverified/);
    assert.doesNotMatch(notes, /## 0\.1\.9/);
});
