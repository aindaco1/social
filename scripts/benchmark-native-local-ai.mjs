#!/usr/bin/env node
// Real inference through the shipping helper, independently of the WebView.
// A packaged --app runs the exact signed helper, library and bundled model.
import { spawnSync, spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import assert from 'node:assert/strict';
import { CONTROL_ARGUMENT, NETWORK_DENIAL_PROFILE, assertNetworkControls, probeNetworkControls, runNetworkDenied } from './lib/macos-network-denial.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2);
const value = flag => args.includes(flag) ? args[args.indexOf(flag) + 1] : null;
const app = value('--app');
const denyNetwork = args.includes('--deny-network');
const scope = 'native_litert_helper_only';
const sha256 = file => createHash('sha256').update(readFileSync(file)).digest('hex');

async function benchmark() {
    assert.equal(process.platform, 'darwin', 'Native LiteRT benchmark requires macOS');
    for (let index = 0; index < args.length; index++) {
        const flag = args[index];
        if (flag === '--deny-network') continue;
        assert.ok(['--app', '--output', CONTROL_ARGUMENT].includes(flag), `Unknown argument: ${flag}`);
        assert.ok(args[index + 1] && !args[index + 1].startsWith('--'), `Missing value for ${flag}`);
        assert.equal(args.indexOf(flag), index, `Duplicate argument: ${flag}`);
        index++;
    }
    if (denyNetwork && !args.includes(CONTROL_ARGUMENT)) {
        // Keep the controls, inference, cancellation and malformed-input checks in
        // one sandboxed process family. Only the parent writes the final report.
        const childArgs = [...args];
        const outputIndex = childArgs.indexOf('--output');
        if (outputIndex !== -1) childArgs.splice(outputIndex, 2);
        const result = await runNetworkDenied(process.execPath, [fileURLToPath(import.meta.url), ...childArgs]);
        const report = JSON.parse(result.stdout);
        assert.equal(report.ok, true);
        assert.equal(report.scope, scope);
        assertNetworkControls(report.network_controls.before, 'blocked');
        assertNetworkControls(report.network_controls.after, 'blocked');
        return { ...report, parent_controls: result.parent_controls };
    }
    assert.ok(!args.includes(CONTROL_ARGUMENT) || denyNetwork, 'Network controls require --deny-network');
    const controls = denyNetwork ? JSON.parse(value(CONTROL_ARGUMENT)) : null;
    if (denyNetwork) assert.ok(Array.isArray(controls), 'Network-denied runs require real controls');
    const blockedBefore = controls ? await probeNetworkControls(controls) : null;
    if (blockedBefore) assertNetworkControls(blockedBefore, 'blocked');
    const helper = app ? path.join(app, 'Contents/MacOS/social-litert') : path.join(root, 'src-tauri/binaries/social-litert-aarch64-apple-darwin');
    const modelRoot = app ? path.join(app, 'Contents/Resources/litert-models') : path.join(root, 'resources/desktop/public/litert/models');
    const manifest = JSON.parse(readFileSync(path.join(modelRoot, 'manifest.json'), 'utf8'));
    const modelFile = manifest.models.find(model => model.id === 'real-esrgan-x4plus-w8a8')?.files.find(file => file.kind === 'model');
    assert.ok(modelFile, 'Bundled model manifest must describe the upscaler');
    const model = path.resolve(modelRoot, modelFile.path);
    assert.ok(model.startsWith(path.resolve(modelRoot) + path.sep), 'Model must remain inside its bundle');
    assert.ok(existsSync(helper), 'Prepare the native helper first');
    assert.equal(sha256(model), modelFile.sha256, 'Model bytes must match the canonical manifest');
    const helperHash = sha256(helper);
    if (app) {
        const signature = spawnSync('/usr/bin/codesign', ['--verify', '--deep', '--strict', app], { encoding: 'utf8', timeout: 30000 });
        assert.equal(signature.status, 0, signature.stderr || 'Packaged signature verification failed');
    }
    const expectedHash = '764150e56d995598177059d5da995be1971a61915e74f0d83968db1c8ef12fe4';
    const input = Buffer.from(Uint8Array.from({ length: 128 * 128 * 3 }, (_, i) => (i * 17 + Math.floor(i / 384)) % 256));
    const runs = [];
    for (const threads of [1, 4]) {
        const result = spawnSync(helper, [model, String(threads)], { env: {}, input: Buffer.concat([Buffer.from('T'), input, Buffer.from('Q')]), timeout: 60000, maxBuffer: 2 * 1024 * 1024 });
        assert.equal(result.status, 0, result.stderr?.toString());
        assert.equal(result.stdout.length, 20 + 4 + 512 * 512 * 3);
        assert.equal(result.stdout.readUInt32LE(0), 0x3154524c);
        assert.equal(result.stdout.readUInt32LE(4), input.length);
        assert.equal(result.stdout.readUInt32LE(8), 512 * 512 * 3);
        assert.equal(result.stdout.readUInt32LE(16), threads);
        const hash = createHash('sha256').update(result.stdout.subarray(24)).digest('hex');
        assert.equal(hash, expectedHash, 'Native output must match the established Wasm reference tile');
        runs.push({ threads, compile_ms: result.stdout.readUInt32LE(12), inference_ms: result.stdout.readUInt32LE(20), output_sha256: hash, network_denied: denyNetwork });
    }
    // Actual process interruption and reaping, not merely an AbortSignal mock.
    const child = spawn(helper, [model, '4'], { env: {}, stdio: ['pipe', 'pipe', 'ignore'] });
    await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { child.kill('SIGKILL'); reject(new Error('Native cancellation test timed out')); }, 10000);
        let readyBytes = 0;
        let started = false;
        let cancelTimer;
        child.stdin.on('error', reject);
        child.once('error', error => { clearTimeout(timeout); reject(error); });
        child.stdout.on('data', data => {
            readyBytes += data.length;
            if (readyBytes >= 20 && !started) {
                started = true;
                child.stdin.write(Buffer.concat([Buffer.from('T'), input]));
                cancelTimer = setTimeout(() => child.kill('SIGKILL'), 50);
            }
        });
        child.once('close', (_, signal) => {
            clearTimeout(timeout); clearTimeout(cancelTimer);
            try { assert.equal(started, true); assert.equal(signal, 'SIGKILL'); resolve(); } catch (error) { reject(error); }
        });
    });
    const malformed = spawnSync(helper, [model, '4'], { env: {}, input: Buffer.from('X'), timeout: 10000 });
    assert.equal(malformed.status, 1);
    const blockedAfter = controls ? await probeNetworkControls(controls) : null;
    if (blockedAfter) assertNetworkControls(blockedAfter, 'blocked');
    assert.equal(sha256(helper), helperHash, 'Helper bytes changed during the test');
    assert.equal(sha256(model), modelFile.sha256, 'Model bytes changed during the test');
    return {
        ok: true, date: new Date().toISOString(), scope, full_app_offline_accepted: false,
        packaged: Boolean(app), helper_sha256: helperHash, model_sha256: modelFile.sha256,
        runs, cancellation_reaped: true, malformed_frame_rejected: true,
        ...(controls ? { sandbox_profile: NETWORK_DENIAL_PROFILE, network_controls: { before: blockedBefore, after: blockedAfter } } : {}),
    };
}

try {
    const report = await benchmark();
    if (value('--output')) writeFileSync(value('--output'), JSON.stringify(report, null, 2) + '\n');
    console.log(JSON.stringify(report, null, 2));
} catch (error) {
    const report = { ok: false, date: new Date().toISOString(), scope, full_app_offline_accepted: false, error: error.message };
    if (value('--output')) writeFileSync(value('--output'), JSON.stringify(report, null, 2) + '\n');
    console.error(JSON.stringify(report, null, 2));
    process.exitCode = 1;
}
