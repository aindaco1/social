import test from 'node:test';
import assert from 'node:assert/strict';
import { createLocalAiSession } from '../resources/desktop/src/localAiSession.js';
import { localAiTiles, localAiTileInput } from '../resources/desktop/src/localAiTiling.js';

const model = { id: 'upscale', feature: 'image_upscaling', runtime: 'litert', files: [{ kind: 'model', path: 'model.tflite' }] };
const fetchAsset = async () => ({ ok: true, json: async () => ({ models: [model] }) });
const compile = client => client.request('compile', { assetsBase: 'https://app.invalid/litert/' });
const tick = () => new Promise(resolve => setImmediate(resolve));
function fixture(handler) {
    const calls = [];
    const controller = new AbortController();
    const fallback = { calls: [], disposed: 0, request(...args) { this.calls.push(args); return 'wasm'; }, dispose() { this.disposed++; } };
    const client = createLocalAiSession({ id: 'test-session', signal: controller.signal, fetchAsset,
        createFallback: () => fallback,
        invoke: async (command, args) => { calls.push({ command, args }); return handler?.(command, args); },
    });
    return { client, controller, calls, fallback };
}

test('native compile, binary tile and release share canonical metadata without browser capability requirements', async () => {
    const f = fixture(command => command.endsWith('start') ? { runtime_version: '2.1.6', cpu_threads: 4 } : command.endsWith('tile') ? new Uint8Array(4 + 512 * 512 * 3).buffer : null);
    const ready = await compile(f.client);
    assert.equal(ready.accelerator, 'native_cpu');
    assert.equal(ready.upscalingModel.id, model.id);
    assert.equal(ready.execution.cpu_threads, 4);
    assert.equal(ready.execution.shared_memory, undefined);
    assert.equal((await f.client.request('tile', { inputBytes: new Uint8Array(128 * 128 * 3) })).outputData.length, 512 * 512 * 3);
    await f.client.request('release'); f.client.dispose();
    assert.equal(f.calls.filter(call => call.command.endsWith('close')).length, 1);
    assert.equal(f.fallback.calls.length, 0);
});

test('older-system unavailability automatically uses the existing cancellable Wasm path', async () => {
    const f = fixture(() => null);
    assert.equal(await compile(f.client), 'wasm');
    assert.equal(await f.client.request('tile', { inputBytes: new Uint8Array(1) }), 'wasm');
    f.controller.abort();
    assert.equal(f.fallback.disposed, 1);
});

test('model or native failures are errors, never ordinary resizing or hidden fallback', async () => {
    const f = fixture(command => { if (command.endsWith('start')) throw new Error('integrity failure'); });
    await assert.rejects(compile(f.client), /integrity failure/);
    assert.equal(f.fallback.calls.length, 0);
    assert.ok(f.calls.some(call => call.command.endsWith('close')));
});

test('cancellation rejects immediately and closes a late native startup again', async () => {
    let resolveStart;
    const f = fixture(command => command.endsWith('start') ? new Promise(resolve => { resolveStart = resolve; }) : null);
    const work = compile(f.client);
    await tick(); f.controller.abort();
    await assert.rejects(work, { name: 'AbortError' });
    assert.equal(f.calls.filter(call => call.command.endsWith('close')).length, 1);
    resolveStart({ runtime_version: '2.1.6', cpu_threads: 4 });
    await tick();
    assert.equal(f.calls.filter(call => call.command.endsWith('close')).length, 2);
    assert.equal(f.fallback.calls.length, 0);
});

test('cancel during native inference closes the process and a fresh session can retry', async () => {
    const f = fixture(command => command.endsWith('start') ? { cpu_threads: 4 } : command.endsWith('tile') ? new Promise(() => {}) : null);
    await compile(f.client);
    const work = f.client.request('tile', { inputBytes: new Uint8Array(128 * 128 * 3) });
    await assert.rejects(f.client.request('release'), /already processing/);
    f.controller.abort();
    await assert.rejects(work, { name: 'AbortError' });
    assert.ok(f.calls.some(call => call.command.endsWith('close')));
    const retry = fixture(command => command.endsWith('start') ? { cpu_threads: 4 } : null);
    assert.equal((await compile(retry.client)).accelerator, 'native_cpu'); retry.client.dispose();
});

test('malformed native output fails before it can become an image', async () => {
    const f = fixture(command => command.endsWith('start') ? { cpu_threads: 4 } : new Uint8Array(4));
    await compile(f.client);
    await assert.rejects(f.client.request('tile', { inputBytes: new Uint8Array(128 * 128 * 3) }), /invalid image tile/);
});

test('halo tiling covers every output pixel once and retains full source dimensions', () => {
    for (const [width, height] of [[1, 1], [32, 32], [128, 128], [129, 97], [257, 191], [512, 512]]) {
        const coverage = new Uint8Array(width * height);
        for (const tile of localAiTiles(width, height)) {
            assert.ok(tile.width + tile.offsetX <= 128);
            assert.ok(tile.height + tile.offsetY <= 128);
            for (let y = tile.y; y < tile.y + tile.height; y++) for (let x = tile.x; x < tile.x + tile.width; x++) coverage[y * width + x]++;
        }
        assert.ok(coverage.every(count => count === 1));
    }
    assert.equal(localAiTiles(128, 128).length, 1);
    assert.equal(localAiTiles(512, 512).length, 36);
    for (const size of [0, -1, 513, 1.5, NaN]) assert.throws(() => localAiTiles(size, 1), /will not shrink/);
});

test('neighboring tiles include real shared context and image-edge padding stays bounded', () => {
    const width = 200, height = 1;
    const pixels = Uint8ClampedArray.from({ length: width * 4 }, (_, i) => Math.floor(i / 4));
    const tiles = localAiTiles(width, height);
    const first = localAiTileInput(pixels, width, height, tiles[0]);
    const second = localAiTileInput(pixels, width, height, tiles[1]);
    assert.equal(first[0], 0);
    assert.equal(first[127 * 3], 111);
    assert.equal(second[0], 80);
    assert.equal(second[16 * 3], 96);
    assert.throws(() => localAiTileInput(new Uint8Array(0), width, height, tiles[0]), /Invalid/);
});
