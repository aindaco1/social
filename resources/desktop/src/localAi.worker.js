// Keep this entry classic: LiteRT uses importScripts for its bundled Wasm glue.
// Dynamic imports work in classic workers in dev; Vite inlines them into an IIFE
// for packaging. No model or runtime code executes on the WebView UI thread.
const runtimePromise = Promise.all([import('@litertjs/core'), import('./localAiModelRuntime.js')])
    .then(([litert, { createLocalAiModelRuntime }]) => createLocalAiModelRuntime({
        litert,
        webgpu: Boolean(navigator.gpu),
        crossOriginIsolated: self.crossOriginIsolated === true,
        hardwareConcurrency: navigator.hardwareConcurrency,
    }));
let busy = false;
self.onmessage = async ({ data: { id, type, payload } }) => {
    if (busy) {
        self.postMessage({ id, ok: false, error: 'Local AI worker is already processing' });
        return;
    }
    busy = true;
    try {
        const runtime = await runtimePromise;
        let result;
        if (type === 'compile') {
            const base = new URL(payload.assetsBase);
            if (base.protocol !== self.location.protocol || base.host !== self.location.host || base.pathname !== '/litert/') {
                throw new Error('Local AI assets must come from the bundled app');
            }
            result = await runtime.compile(base.href);
        } else if (type === 'tile') {
            result = await runtime.tile(payload.inputBytes);
        } else if (type === 'release') {
            runtime.release();
            result = null;
        } else {
            throw new Error('Unknown Local AI worker action');
        }
        self.postMessage({ id, ok: true, result }, result?.outputData ? [result.outputData.buffer] : []);
    } catch (error) {
        self.postMessage({ id, ok: false, error: String(error) });
    } finally {
        busy = false;
    }
};
