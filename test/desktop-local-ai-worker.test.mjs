import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createLocalAiWorkerClient, observeLocalAiResponsiveness } from '../resources/desktop/src/localAiWorkerClient.js';
import { createLocalAiModelRuntime, loadLocalAiUpscalingBundle, localAiWasmModule, localAiExecutionOptions } from '../resources/desktop/src/localAiModelRuntime.js';
import { createLocalMediaOperation } from '../resources/desktop/src/localMediaOperation.js';

function clientFixture() {
    const controller = new AbortController();
    const worker = { messages: [], terminated: 0, postMessage(...args) { this.messages.push(args); }, terminate() { this.terminated++; } };
    const client = createLocalAiWorkerClient({ createWorker: () => worker, signal: controller.signal });
    const reply = (result, id = worker.messages.at(-1)[0].id) => worker.onmessage?.({ data: { id, ok: true, result } });
    return { worker, controller, client, reply };
}

test('worker client serializes requests, transfers pixels and ignores stale replies', async () => {
    const f = clientFixture();
    const inputBytes = new Uint8Array(3);
    const pending = f.client.request('tile', { inputBytes }, [inputBytes.buffer]);
    assert.deepEqual(f.worker.messages[0][1], [inputBytes.buffer]);
    await assert.rejects(f.client.request('tile'), /already processing/);
    f.reply('stale', 0);
    f.reply('pixels');
    assert.equal(await pending, 'pixels');
    f.client.dispose();
    f.client.dispose();
    assert.equal(f.worker.terminated, 1);
    await assert.rejects(f.client.request('compile'), /closed/);
});

test('cancel terminates in-flight CPU work immediately without waiting for a worker reply', async () => {
    const f = clientFixture();
    const pending = f.client.request('tile');
    const staleHandler = f.worker.onmessage;
    f.controller.abort();
    await assert.rejects(pending, { name: 'AbortError' });
    assert.equal(f.worker.terminated, 1);
    assert.equal(f.worker.onmessage, null);
    staleHandler({ data: { id: 1, ok: true, result: 'late pixels' } });
    await assert.rejects(f.client.request('tile'), { name: 'AbortError' });
    f.client.dispose();
    assert.equal(f.worker.terminated, 1);
});

test('already canceled work never creates a worker', () => {
    const controller = new AbortController();
    controller.abort();
    assert.throws(() => createLocalAiWorkerClient({ createWorker: () => assert.fail('worker started'), signal: controller.signal }), { name: 'AbortError' });
});

for (const failure of ['error', 'messageerror', 'invalid', 'runtime', 'post']) {
    test(`worker ${failure} failure rejects work and releases the worker`, async () => {
        const f = clientFixture();
        if (failure === 'post') f.worker.postMessage = () => { throw new Error('post failed'); };
        const pending = f.client.request('compile');
        if (failure === 'error') f.worker.onerror({ message: 'script failed', preventDefault() {} });
        if (failure === 'messageerror') f.worker.onmessageerror();
        if (failure === 'invalid') f.worker.onmessage({ data: { id: 1 } });
        if (failure === 'runtime') f.worker.onmessage({ data: { id: 1, ok: false, error: 'model failed' } });
        await assert.rejects(pending);
        assert.equal(f.worker.terminated, 1);
    });
}

test('shared operation cancels worker before commit and a fresh retry can save once', async () => {
    let state;
    let worker;
    let saves = 0;
    const operation = createLocalMediaOperation((value) => { state = value; });
    const action = async ({ signal, beginCommit }) => {
        worker = { postMessage() {}, terminate() { this.terminated = true; } };
        const client = createLocalAiWorkerClient({ createWorker: () => worker, signal });
        try {
            await client.request('tile');
            beginCommit();
            saves++;
            assert.equal(operation.cancel(), false);
            return { derivative: { name: 'result.png' } };
        } finally { client.dispose(); }
    };
    const canceled = operation.run('Upscale', action, { cancellable: true });
    operation.cancel();
    assert.equal(await canceled, false);
    assert.equal(state.result.status, 'canceled');
    assert.equal(worker.terminated, true);
    assert.equal(saves, 0);
    const retry = operation.run('Upscale', action, { cancellable: true });
    worker.onmessage({ data: { id: 1, ok: true, result: null } });
    assert.equal(await retry, true);
    assert.equal(state.result.status, 'complete');
    assert.equal(worker.terminated, true);
    assert.equal(saves, 1);
});

const model = { id: 'test', feature: 'image_upscaling', runtime: 'litert', files: [{ kind: 'model', path: 'model.tflite' }], inputs: { image: { shape: [1, 128, 128, 3] } }, outputs: { upscaled_image: { shape: [1, 512, 512, 3] } } };
function runtimeFixture({ webgpu = true, jspi = false, fullyAccelerated = true, failCompile = false, failRun = false } = {}) {
    const calls = [];
    const moduleScope = {};
    const tensors = [];
    const bytes = new Uint8Array([0, 1, 2]);
    const tensor = (accelerator = 'wasm') => {
        const value = { accelerator, deleted: false, delete() { assert.equal(this.deleted, false); this.deleted = true; }, toTypedArray: () => bytes, moveTo: async () => tensor() };
        tensors.push(value);
        return value;
    };
    const compiledModel = { deleted: false, isFullyAccelerated: fullyAccelerated, delete() { this.deleted = true; }, async run() { if (failRun) throw new Error('inference failed'); return [tensor(jspi ? 'webgpu' : 'wasm')]; } };
    const litert = {
        supportsFeature: async (feature) => feature === 'jspi' ? jspi : true,
        async loadLiteRt(...args) {
            assert.equal(moduleScope.Module.locateFile('litert_wasm_internal.wasm'), 'tauri://localhost/litert/wasm/litert_wasm_internal.wasm');
            calls.push(['load', ...args]);
        },
        async loadAndCompile(...args) { calls.push(['compile', ...args]); if (failCompile) throw new Error('compile failed'); return compiledModel; },
        unloadLiteRt() { calls.push(['unload']); },
        Tensor: class { constructor() { return tensor(); } },
    };
    let clock = 0;
    const runtime = createLocalAiModelRuntime({ litert, webgpu, moduleScope, now: () => ++clock * 10, fetchAsset: async () => ({ ok: true, json: async () => ({ models: [model] }) }) });
    return { runtime, calls, tensors, bytes, compiledModel };
}

for (const [options, accelerator, requested] of [
    [{}, 'wasm_fallback', 'wasm'],
    [{ webgpu: false }, 'wasm', 'wasm'],
    [{ jspi: true }, 'webgpu', ['webgpu', 'wasm']],
    [{ jspi: true, fullyAccelerated: false }, 'wasm_fallback', ['webgpu', 'wasm']],
]) {
    test(`worker runtime selects ${accelerator} with capability-checked readback`, async () => {
        const f = runtimeFixture(options);
        const info = await f.runtime.compile('tauri://localhost/litert/');
        assert.equal(info.accelerator, accelerator);
        assert.equal(info.compile_ms, 10);
        assert.deepEqual(f.calls[0], ['load', 'tauri://localhost/litert/wasm/', { jspi: !!options.jspi, threads: false }]);
        assert.deepEqual(f.calls[1], ['compile', 'tauri://localhost/litert/models/model.tflite', { accelerator: requested, cpuOptions: { numThreads: 1 } }]);
        const output = await f.runtime.tile(new Uint8Array(128 * 128 * 3));
        assert.deepEqual(output.outputData, f.bytes);
        assert.notEqual(output.outputData, f.bytes);
        assert.equal(output.inference_ms, 10);
        assert.ok(f.tensors.every((tensor) => tensor.deleted));
        f.runtime.release();
        f.runtime.release();
        assert.equal(f.compiledModel.deleted, true);
        assert.equal(f.calls.filter(([action]) => action === 'unload').length, 1);
    });
}

test('runtime releases failed compilation and failed-inference input tensors', async () => {
    const compilation = runtimeFixture({ failCompile: true });
    await assert.rejects(compilation.runtime.compile('tauri://localhost/litert/'), /compile failed/);
    assert.equal(compilation.calls.at(-1)[0], 'unload');
    const inference = runtimeFixture({ failRun: true });
    await assert.rejects(inference.runtime.tile(new Uint8Array()), /not ready/);
    await inference.runtime.compile('tauri://localhost/litert/');
    await assert.rejects(inference.runtime.tile(new Uint8Array()), /invalid/);
    await assert.rejects(inference.runtime.tile(new Uint8Array(128 * 128 * 3)), /inference failed/);
    assert.ok(inference.tensors.every((tensor) => tensor.deleted));
    inference.runtime.release();
    assert.equal(inference.compiledModel.deleted, true);
});

test('bundled model manifest rejects missing, remote and parent-relative model paths', async () => {
    for (const path of ['', 'https://example.com/model.tflite', '../model.tflite', '/model.tflite']) {
        await assert.rejects(loadLocalAiUpscalingBundle('tauri://localhost/litert/', async () => ({ ok: true, json: async () => ({ models: [{ ...model, files: [{ kind: 'model', path }] }] }) })), /not listed/);
    }
});

test('worker runtime locator resolves only reviewed Wasm filenames beside the bundled glue', () => {
    for (const base of ['tauri://localhost/litert/', 'http://127.0.0.1:1420/litert/']) {
        const module = localAiWasmModule(base);
        assert.equal(module.mainScriptUrlOrBlob, undefined);
        assert.equal(localAiWasmModule(base, true).mainScriptUrlOrBlob, `${base}wasm/litert_wasm_threaded_internal.js`);
        for (const variant of ['', 'compat_', 'jspi_', 'threaded_']) {
            const name = `litert_wasm_${variant}internal.wasm`;
            assert.equal(module.locateFile(name), `${base}wasm/${name}`);
        }
        for (const name of ['../runtime.wasm', 'https://example.com/a.wasm', 'unreviewed.wasm']) {
            assert.throws(() => module.locateFile(name), /Unknown bundled/);
        }
    }
});

test('threaded CPU selection is bounded and requires isolation, memory, and SIMD together', async () => {
    for (const [cores, expected] of [[1, 1], [2, 2], [4, 2], [10, 4], [64, 4], [NaN, 1]]) {
        const options = await localAiExecutionOptions({ supportsFeature: async () => true }, { webgpu: false, crossOriginIsolated: true, hardwareConcurrency: cores });
        assert.equal(options.compileOptions.cpuOptions.numThreads, expected);
        assert.equal(options.loadOptions.threads, expected > 1);
        assert.equal(options.loadOptions.jspi, false);
    }
    for (const missing of ['relaxedSimd', 'threads', 'isolation']) {
        const options = await localAiExecutionOptions({ supportsFeature: async (feature) => feature !== missing }, { webgpu: false, crossOriginIsolated: missing !== 'isolation', hardwareConcurrency: 10 });
        assert.equal(options.loadOptions.threads, false);
        assert.equal(options.compileOptions.cpuOptions.numThreads, 1);
    }
});

test('GPU selection never combines JSPI with threads and requires the actual JSPI runtime variant', async () => {
    const options = await localAiExecutionOptions({ supportsFeature: async () => true }, { webgpu: true, crossOriginIsolated: true, hardwareConcurrency: 10 });
    assert.deepEqual(options.loadOptions, { jspi: true, threads: false });
    assert.deepEqual(options.compileOptions.accelerator, ['webgpu', 'wasm']);
    const compat = await localAiExecutionOptions({ supportsFeature: async (feature) => feature !== 'relaxedSimd' }, { webgpu: true, crossOriginIsolated: true, hardwareConcurrency: 10 });
    assert.deepEqual(compat.loadOptions, { jspi: false, threads: false });
    assert.equal(compat.compileOptions.accelerator, 'wasm');
});

test('UI timer diagnostics measure observed gaps and stop once without persistent logging', () => {
    let time = 1000;
    let tick;
    let stopped = 0;
    const finish = observeLocalAiResponsiveness({ now: () => time, schedule(callback, interval) { tick = callback; assert.equal(interval, 100); return 42; }, unschedule(id) { assert.equal(id, 42); stopped++; } });
    time += 100; tick();
    time += 140; tick();
    time += 25;
    assert.deepEqual(finish(), { elapsed_ms: 265, ui_max_gap_ms: 140, ui_samples: 3 });
    finish();
    assert.equal(stopped, 1);
});
