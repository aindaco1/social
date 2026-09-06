import { loadLocalAiUpscalingBundle } from './localAiModelRuntime.js';
import { throwIfLocalMediaCanceled } from './localMediaOperation.js';

// Native and browser execution share model metadata, pixels, progress and saves.
// Only explicit native unavailability selects Wasm; failed inference is never
// replaced with ordinary resizing or silently retried through another engine.
export function createLocalAiSession({ invoke, createFallback, signal, id = crypto.randomUUID(), fetchAsset = fetch, now = () => performance.now() }) {
    throwIfLocalMediaCanceled(signal);
    let closed = false;
    let pending = false;
    let nativeStarted = false;
    let ready = false;
    let fallback;
    let rejectPending;
    const closeNative = () => invoke('local_ai_native_close', { sessionId: id });
    const dispose = (error = new Error('Local AI session closed')) => {
        if (closed) return;
        closed = true;
        signal?.removeEventListener('abort', abort);
        fallback?.dispose();
        if (nativeStarted) void closeNative().catch(() => {});
        rejectPending?.(error);
    };
    const abort = () => {
        try { throwIfLocalMediaCanceled(signal); } catch (error) { dispose(error); }
    };
    signal?.addEventListener('abort', abort, { once: true });

    const run = async (type, payload, transfer) => {
        if (fallback) return fallback.request(type, payload, transfer);
        if (type === 'compile') {
            if (ready) throw new Error('Local AI model is already loaded');
            const started = now();
            const bundle = await loadLocalAiUpscalingBundle(payload.assetsBase, (url, options) => fetchAsset(url, { ...options, signal }));
            if (closed) throw new Error('Local AI session closed');
            nativeStarted = true;
            let native;
            try {
                native = await invoke('local_ai_native_start', { sessionId: id });
            } finally {
                // Cancel can reach Rust before startup has registered the session.
                // Always close a late response, including a rejected startup.
                if (closed) await closeNative();
            }
            if (closed) throw new Error('Local AI session closed');
            if (!native) {
                nativeStarted = false;
                fallback = createFallback();
                return fallback.request(type, payload, transfer);
            }
            ready = true;
            return {
                upscalingModel: bundle.upscalingModel,
                models: bundle.models.length,
                outputTileSize: bundle.outputTileSize,
                accelerator: 'native_cpu', webgpu: false,
                compile_ms: Math.round(now() - started),
                execution: { backend: 'native_litert', runtime_version: native.runtime_version, cpu_threads: native.cpu_threads, threaded: native.cpu_threads > 1 },
            };
        }
        if (type === 'release') {
            if (nativeStarted) await closeNative();
            nativeStarted = ready = false;
            return;
        }
        if (type !== 'tile' || !ready) throw new Error('Local AI model is not ready');
        if (!(payload.inputBytes instanceof Uint8Array) || payload.inputBytes.length !== 128 * 128 * 3) throw new Error('Local AI input tile is invalid');
        const response = await invoke('local_ai_native_tile', { sessionId: id, input: Array.from(payload.inputBytes) });
        const bytes = response instanceof Uint8Array ? response : new Uint8Array(response);
        if (bytes.length !== 4 + 512 * 512 * 3) throw new Error('Local AI returned an invalid image tile');
        return { inference_ms: new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0, true), outputData: bytes.subarray(4) };
    };
    return {
        request(type, payload = {}, transfer = []) {
            try { throwIfLocalMediaCanceled(signal); } catch (error) { return Promise.reject(error); }
            if (closed) return Promise.reject(new Error('Local AI session closed'));
            if (pending) return Promise.reject(new Error('Local AI is already processing'));
            pending = true;
            return Promise.race([run(type, payload, transfer), new Promise((_, reject) => { rejectPending = reject; })])
                .catch((error) => { dispose(error); throw error; })
                .finally(() => { pending = false; rejectPending = null; });
        },
        dispose,
    };
}
