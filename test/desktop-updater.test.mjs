import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import { createUpdateProgress } from '../shared/dust-wave-platform/packages/desktop-core/src/update-progress.js';

// Exercise the app's actual callbacks with the same fixture before and after
// extraction. Native signature/version checks remain in the Rust command.
const source = readFileSync(process.env.SOCIAL_UPDATE_SOURCE
    || new URL('../resources/desktop/src/App.vue', import.meta.url), 'utf8');
const callbacks = source.slice(source.indexOf('const updaterErrorMessage ='),
    source.indexOf('const checkOrInstallSoftwareUpdate ='));
function fixture({ check = async () => null, install = async () => {} } = {}) {
    const state = Object.fromEntries([
        'Checking', 'CheckingSilently', 'LastCheckWasAutomatic', 'Installing',
        'Status', 'Error', 'Progress', 'Available',
    ].map(key => ['softwareUpdate' + key, { value: null }]));
    const api = vm.runInNewContext(callbacks + '\n({ checkSoftwareUpdate, installSoftwareUpdate });', {
        ...state, createUpdateProgress, checkForUpdate: check, invoke: install,
        Channel: class {}, Error, formatBytes: value => value + ' B',
    });
    return { ...state, ...api };
}

test('launch check is silent and never installs; manual check exposes failures', async () => {
    let checks = 0;
    const app = fixture({
        check: async options => { checks++; assert.equal(options.timeout, 15000); throw new Error('Offline'); },
        install: () => assert.fail('Checking cannot install'),
    });
    await app.checkSoftwareUpdate({ silent: true });
    assert.equal(app.softwareUpdateStatus.value, 'Automatic update check unavailable');
    assert.equal(app.softwareUpdateLastCheckWasAutomatic.value, true);
    assert.equal(app.softwareUpdateChecking.value, false);
    await app.checkSoftwareUpdate();
    assert.equal(app.softwareUpdateStatus.value, 'Update check unavailable');
    assert.equal(app.softwareUpdateError.value, 'Error: Offline');
    assert.equal(checks, 2);
});

test('explicit install retains version binding, progress, verification and restart states', async () => {
    const app = fixture({ install: async (command, { expectedVersion, onEvent }) => {
        assert.equal(command, 'install_software_update_and_restart');
        assert.equal(expectedVersion, '1.2.3');
        assert.equal(app.softwareUpdateInstalling.value, true);
        onEvent.onmessage({ event: 'Started', data: { contentLength: 100 } });
        assert.equal(app.softwareUpdateProgress.value, 'Downloading 0%');
        onEvent.onmessage({ event: 'Progress', data: { chunkLength: 35 } });
        onEvent.onmessage({ event: 'Progress', data: { chunkLength: 10 } });
        assert.equal(app.softwareUpdateProgress.value, 'Downloading 45% · 45 B of 100 B');
        onEvent.onmessage({ event: 'Started', data: {} });
        onEvent.onmessage({ event: 'Progress', data: { chunkLength: 7 } });
        assert.equal(app.softwareUpdateProgress.value, 'Downloaded 7 B');
        onEvent.onmessage({ event: 'Finished' });
        assert.equal(app.softwareUpdateProgress.value, 'Verifying signed update');
        onEvent.onmessage({ event: 'Installing' });
        assert.equal(app.softwareUpdateStatus.value, 'Installing version 1.2.3');
        onEvent.onmessage({ event: 'Restarting' });
        assert.equal(app.softwareUpdateStatus.value, 'Version 1.2.3 installed');
    } });
    app.softwareUpdateAvailable.value = { version: '1.2.3' };
    await app.installSoftwareUpdate();
    assert.equal(app.softwareUpdateError.value, '');
    assert.equal(app.softwareUpdateInstalling.value, false);
});

test('failed installation leaves the offered update available for an explicit retry', async () => {
    const app = fixture({ install: async () => { throw new Error('Disk full'); } });
    app.softwareUpdateAvailable.value = { version: '1.2.3' };
    await app.installSoftwareUpdate();
    assert.equal(app.softwareUpdateStatus.value, 'Update install failed');
    assert.equal(app.softwareUpdateError.value, 'Error: Disk full');
    assert.equal(app.softwareUpdateAvailable.value.version, '1.2.3');
    assert.equal(app.softwareUpdateInstalling.value, false);
});
