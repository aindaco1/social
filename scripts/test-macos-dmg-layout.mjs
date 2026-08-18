#!/usr/bin/env node

import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, symlink, unlink, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {
    applicationsLinkTargetIsValid,
    validateMacosDmgLayout,
} from './lib/macos-dmg.mjs';

const roots = [];

async function layoutFixture() {
    const root = await mkdtemp(path.join(os.tmpdir(), 'dust-wave-dmg-layout-'));
    roots.push(root);
    await mkdir(path.join(root, 'Dust Wave Social.app'));
    await symlink('/Applications', path.join(root, 'Applications'));
    await writeFile(path.join(root, '.VolumeIcon.icns'), 'icon');
    return root;
}

try {
    assert.equal(applicationsLinkTargetIsValid('/Applications', 'darwin'), true);
    assert.equal(applicationsLinkTargetIsValid('/tmp', 'darwin'), false);

    const valid = await layoutFixture();
    assert.deepEqual(await validateMacosDmgLayout(valid), {
        schemaVersion: 'dust-wave-social-macos-dmg-layout-v1',
        appName: 'Dust Wave Social.app',
        applicationsLink: '/Applications',
        entries: ['.VolumeIcon.icns', 'Applications', 'Dust Wave Social.app'],
    });

    const missingApplications = await layoutFixture();
    await unlink(path.join(missingApplications, 'Applications'));
    await assert.rejects(validateMacosDmgLayout(missingApplications), /entries are invalid/);

    const redirectedApplications = await layoutFixture();
    await unlink(path.join(redirectedApplications, 'Applications'));
    await symlink('/tmp', path.join(redirectedApplications, 'Applications'));
    await assert.rejects(validateMacosDmgLayout(redirectedApplications), /link target is invalid/);

    const missingIcon = await layoutFixture();
    await unlink(path.join(missingIcon, '.VolumeIcon.icns'));
    await assert.rejects(validateMacosDmgLayout(missingIcon), /entries are invalid/);

    const extraEntry = await layoutFixture();
    await writeFile(path.join(extraEntry, 'unexpected.txt'), 'unexpected');
    await assert.rejects(validateMacosDmgLayout(extraEntry), /entries are invalid/);

    console.log('macOS DMG layout tests passed.');
} finally {
    for (const root of roots.reverse()) {
        await rm(root, { recursive: true, force: true });
    }
}
