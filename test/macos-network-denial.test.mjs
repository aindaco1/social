import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { assertNetworkControls, createNetworkControls, probeNetworkControls, runNetworkDenied } from '../scripts/lib/macos-network-denial.mjs';

const kinds = ['tcp4', 'tcp6', 'udp4', 'udp6'];
const blocked = () => kinds.map(kind => ({ kind, outcome: 'blocked', code: 'EPERM' }));

test('network-denied acceptance requires all TCP/UDP and IPv4/IPv6 controls', () => {
    assert.doesNotThrow(() => assertNetworkControls(blocked(), 'blocked'));
    assert.throws(() => assertNetworkControls([], 'blocked'));
    assert.throws(() => assertNetworkControls(blocked().slice(1), 'blocked'));
    assert.throws(() => assertNetworkControls([...blocked().slice(1), blocked()[1]], 'blocked'));
});

test('timeouts, connection errors, and accidental successful traffic cannot pass', () => {
    for (const replacement of [
        { outcome: 'failed', code: 'TIMEOUT' },
        { outcome: 'failed', code: 'ECONNREFUSED' },
        { outcome: 'blocked', code: 'ECONNREFUSED' },
        { outcome: 'connected' },
    ]) {
        const results = blocked();
        results[0] = { ...results[0], ...replacement };
        assert.throws(() => assertNetworkControls(results, 'blocked'));
    }
});

test('ordinary parent process can reach all four live controls', async () => {
    const controls = await createNetworkControls();
    try { assertNetworkControls(await probeNetworkControls(controls.targets), 'connected'); }
    finally { await controls.close(); }
});

test('macOS denies network in child and preserves networking in parent', { skip: process.platform !== 'darwin', timeout: 20000 }, async () => {
    const library = new URL('../scripts/lib/macos-network-denial.mjs', import.meta.url).href;
    const script = `import {probeNetworkControls, assertNetworkControls} from ${JSON.stringify(library)};
        const controls = JSON.parse(process.argv[process.argv.indexOf('--network-controls') + 1]);
        const results = await probeNetworkControls(controls);
        assertNetworkControls(results, 'blocked');
        console.log(JSON.stringify(results));`;
    // -- prevents the internal control argument from being interpreted by Node.
    const result = await runNetworkDenied(process.execPath, ['--input-type=module', '-e', script, '--']);
    assertNetworkControls(JSON.parse(result.stdout), 'blocked');
    assertNetworkControls(result.parent_controls.before, 'connected');
    assertNetworkControls(result.parent_controls.after, 'connected');
});

test('an unsuccessful sandboxed command rejects instead of reporting acceptance', { skip: process.platform !== 'darwin', timeout: 20000 }, async () => {
    await assert.rejects(runNetworkDenied('/usr/bin/false', []), /Sandboxed check failed/);
});

test('a failed CLI run replaces a stale success report and exits nonzero', { skip: process.platform !== 'darwin', timeout: 20000 }, async () => {
    const temporary = await mkdtemp(path.join(os.tmpdir(), 'social-native-offline-failure-'));
    const output = path.join(temporary, 'report.json');
    try {
        await writeFile(output, JSON.stringify({ ok: true }));
        const result = spawnSync(process.execPath, [
            fileURLToPath(new URL('../scripts/benchmark-native-local-ai.mjs', import.meta.url)),
            '--deny-network', '--app', path.join(temporary, 'missing.app'), '--output', output,
        ], { encoding: 'utf8', timeout: 15000 });
        assert.equal(result.status, 1, result.stderr);
        const report = JSON.parse(await readFile(output, 'utf8'));
        assert.equal(report.ok, false);
        assert.equal(report.scope, 'native_litert_helper_only');
        assert.equal(report.full_app_offline_accepted, false);
        assert.match(report.error, /ENOENT/);
    } finally { await rm(temporary, { recursive: true, force: true }); }
});
