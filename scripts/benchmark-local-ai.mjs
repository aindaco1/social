#!/usr/bin/env node
// CPU diagnostic, not WebKit acceptance. Uses the shipped runtime/model and the
// shared model owner; no network, app data, or provider credentials are needed.
import { readFileSync, mkdtempSync, copyFileSync, unlinkSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import * as litert from '@litertjs/core';
import { createLocalAiModelRuntime } from '../resources/desktop/src/localAiModelRuntime.js';

const script = fileURLToPath(import.meta.url);
const root = new URL('../resources/desktop/public/litert/', import.meta.url);
const cliArgs = process.argv.slice(2);
const threadArg = cliArgs.find((arg) => /^--threads=/.test(arg));
const threads = threadArg ? Number(threadArg.split('=')[1]) : 1;
if (![1, 2, 4].includes(threads) || cliArgs.some((arg) => !['--child', '--compat', '--threads=1', '--threads=2', '--threads=4'].includes(arg)) || (threads > 1 && cliArgs.includes('--compat'))) {
    throw new Error('Usage: node scripts/benchmark-local-ai.mjs [--compat | --threads=1|2|4]');
}
if (!process.argv.includes('--child')) {
    // The parent owns this mechanical CommonJS copy and removes it even when
    // the CPU-bound child hits the timeout and cannot execute its own finally.
    const adapterDirectory = threads > 1 ? mkdtempSync(path.join(tmpdir(), 'social-litert-benchmark-')) : null;
    const threadEntry = adapterDirectory ? path.join(adapterDirectory, 'litert-threaded.cjs') : '';
    let result;
    try {
        if (threadEntry) copyFileSync(new URL('wasm/litert_wasm_threaded_internal.js', root), threadEntry);
        result = spawnSync(process.execPath, [script, '--child', ...process.argv.slice(2)], {
            stdio: 'inherit', timeout: 240_000, killSignal: 'SIGKILL',
            env: { ...process.env, SOCIAL_BENCHMARK_THREAD_ENTRY: threadEntry },
        });
    } finally {
        if (threadEntry) { unlinkSync(threadEntry); rmdirSync(adapterDirectory); }
    }
    if (result.error) console.error(result.error.message);
    process.exit(result.status ?? 1);
}

const require = createRequire(import.meta.url);
const threadEntry = threads > 1 ? process.env.SOCIAL_BENCHMARK_THREAD_ENTRY : null;
if (threads > 1 && !threadEntry) throw new Error('Threaded diagnostic must run through its bounded parent process');
globalThis.self = globalThis;
globalThis.importScripts = (url) => {
    const filename = fileURLToPath(url);
    // Execute the unmodified Emscripten CommonJS-capable loader in Node. This is
    // a CLI-only adapter, never shipped to the app or used to relax its CSP.
    const factory = new Function('require', '__dirname', '__filename',
        `${readFileSync(filename, 'utf8')}\nreturn ModuleFactory;`);
    self.ModuleFactory = factory(require, path.dirname(filename), filename);
};
const runtime = createLocalAiModelRuntime({
    webgpu: false,
    crossOriginIsolated: threads > 1,
    hardwareConcurrency: threads * 2,
    litert: {
        ...litert,
        supportsFeature: (feature) => feature === 'jspi' ? false : litert.supportsFeature(feature),
        loadLiteRt: (url, options) => {
            if (threadEntry) self.Module.mainScriptUrlOrBlob = threadEntry;
            return litert.loadLiteRt(process.argv.includes('--compat') ? new URL('litert_wasm_compat_internal.js', url).href : url, options);
        },
        loadAndCompile: (url, options) => litert.loadAndCompile(new Uint8Array(readFileSync(new URL(url))), options),
    },
    fetchAsset: async (url) => ({ ok: true, json: async () => JSON.parse(readFileSync(new URL(url), 'utf8')) }),
});
try {
    const compiled = await runtime.compile(root.href);
    console.log(JSON.stringify({ event: 'compiled', engine: process.version, compat: process.argv.includes('--compat'), compile_ms: compiled.compile_ms, model: compiled.upscalingModel.id, execution: compiled.execution }));
    // Fixed full-tile RGB gradient; intentionally independent of private media.
    const pixels = Uint8Array.from({ length: 128 * 128 * 3 }, (_, i) => (i * 17 + Math.floor(i / 384)) % 256);
    const cpuStarted = process.cpuUsage();
    const output = await runtime.tile(pixels);
    console.log(JSON.stringify({
        event: 'inference', inference_ms: output.inference_ms,
        cpu: process.cpuUsage(cpuStarted), output_bytes: output.outputData.byteLength,
        output_sha256: createHash('sha256').update(output.outputData).digest('hex'),
    }));
} finally {
    runtime.release();
}
