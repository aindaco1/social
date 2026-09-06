import { LOCAL_AI_UPSCALE_TILE_SIZE, LOCAL_AI_UPSCALE_SCALE_FACTOR } from './localAiConfig.js';

const localAiModelShapeValue = (shape, index, fallback) => {
    const value = Number(Array.isArray(shape) ? shape[index] : fallback);
    return Number.isFinite(value) && value > 0 ? value : fallback;
};

export const loadLocalAiUpscalingBundle = async (assetsBase, fetchAsset = fetch) => {
    const manifestResponse = await fetchAsset(new URL('models/manifest.json', assetsBase).href, { cache: 'no-store' });

    if (!manifestResponse.ok) {
        throw new Error(`Local AI model manifest unavailable (${manifestResponse.status})`);
    }

    const manifest = await manifestResponse.json();
    const models = Array.isArray(manifest?.models) ? manifest.models : [];
    const upscalingModel = models.find((model) => model.feature === 'image_upscaling' && model.runtime === 'litert');
    const modelFile = upscalingModel?.files?.find((file) => file.kind === 'model');

    if (!upscalingModel || !modelFile?.path || !/^[a-zA-Z0-9_./-]+\.tflite$/.test(modelFile.path) || modelFile.path.split('/').includes('..') || modelFile.path.startsWith('/')) {
        throw new Error('Local AI upscaling model is not listed in the bundled manifest');
    }

    const inputShape = upscalingModel.inputs?.image?.shape || [1, LOCAL_AI_UPSCALE_TILE_SIZE, LOCAL_AI_UPSCALE_TILE_SIZE, 3];
    const outputShape = upscalingModel.outputs?.upscaled_image?.shape || [
        1,
        LOCAL_AI_UPSCALE_TILE_SIZE * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        LOCAL_AI_UPSCALE_TILE_SIZE * LOCAL_AI_UPSCALE_SCALE_FACTOR,
        3,
    ];
    const inputTileSize = localAiModelShapeValue(inputShape, 1, LOCAL_AI_UPSCALE_TILE_SIZE);
    const outputTileSize = localAiModelShapeValue(outputShape, 1, inputTileSize * LOCAL_AI_UPSCALE_SCALE_FACTOR);
    const modelScaleFactor = outputTileSize / inputTileSize;

    if (inputTileSize !== LOCAL_AI_UPSCALE_TILE_SIZE || modelScaleFactor !== LOCAL_AI_UPSCALE_SCALE_FACTOR) {
        throw new Error('Bundled Local AI upscaling model must use 128px input tiles and x4 output');
    }

    return {
        manifest,
        models,
        upscalingModel,
        modelFile,
        inputShape,
        outputShape,
        inputTileSize,
        outputTileSize,
        modelScaleFactor,
    };
};

const runLocalAiUpscaleTile = async (compiledModel, Tensor, inputBytes, inputShape) => {
    let inputTensor = null;
    let outputTensor = null;
    let cpuOutputTensor = null;
    let outputs = null;

    try {
        inputTensor = new Tensor(inputBytes, inputShape);
        outputs = await compiledModel.run(inputTensor);
        const outputList = Array.isArray(outputs) ? outputs : Object.values(outputs || {});
        outputTensor = outputList[0];

        if (!outputTensor) {
            throw new Error('Local AI model did not return an output tensor');
        }

        cpuOutputTensor = outputTensor.accelerator === 'wasm' ? outputTensor : await outputTensor.moveTo('wasm');
        const outputData = cpuOutputTensor.toTypedArray();
        return outputData.slice ? outputData.slice() : new outputData.constructor(outputData);
    } finally {
        if (inputTensor && !inputTensor.deleted) {
            inputTensor.delete();
        }

        const outputList = Array.isArray(outputs) ? outputs : Object.values(outputs || {});

        for (const tensor of outputList) {
            if (tensor && tensor !== cpuOutputTensor && !tensor.deleted) {
                tensor.delete();
            }
        }

        if (cpuOutputTensor && !cpuOutputTensor.deleted) {
            cpuOutputTensor.delete();
        }
    }
};


export function localAiWasmModule(assetsBase, threaded = false) {
    return {
        ...(threaded ? { mainScriptUrlOrBlob: new URL('wasm/litert_wasm_threaded_internal.js', assetsBase).href } : {}),
        locateFile(filename) {
            if (!/^litert_wasm_(?:compat_|jspi_|threaded_)?internal\.wasm$/.test(filename)) {
                throw new Error('Unknown bundled Local AI runtime file');
            }
            return new URL(`wasm/${filename}`, assetsBase).href;
        },
    };
}

export async function localAiExecutionOptions(litert, { webgpu, crossOriginIsolated = false, hardwareConcurrency = 1 }) {
    const [jspi, relaxedSimd, sharedMemory] = await Promise.all(
        ['jspi', 'relaxedSimd', 'threads'].map((feature) => litert.supportsFeature(feature)),
    );
    // The threaded bundle requires both shared memory and relaxed SIMD, and
    // cannot be combined with JSPI. Leave CPU capacity for the rest of the app.
    const gpuReadback = Boolean(webgpu && jspi && relaxedSimd);
    const cores = Number.isFinite(hardwareConcurrency) ? Math.max(1, Math.floor(hardwareConcurrency)) : 1;
    const threads = !gpuReadback && crossOriginIsolated && relaxedSimd && sharedMemory && cores > 1;
    const cpuThreads = threads ? Math.min(4, Math.max(2, Math.floor(cores / 2))) : 1;
    return {
        loadOptions: { jspi: gpuReadback, threads: Boolean(threads) },
        compileOptions: { accelerator: gpuReadback ? ['webgpu', 'wasm'] : 'wasm', cpuOptions: { numThreads: cpuThreads } },
        execution: { cpu_threads: cpuThreads, threaded: Boolean(threads), relaxed_simd: relaxedSimd, shared_memory: sharedMemory, cross_origin_isolated: crossOriginIsolated, jspi },
        gpuReadback,
    };
}

export function createLocalAiModelRuntime({ litert, webgpu, crossOriginIsolated = false, hardwareConcurrency = 1, now = () => performance.now(), fetchAsset = fetch, moduleScope = globalThis }) {
    let modelBundle = null;
    let runtimeLoaded = false;
    const release = () => {
        if (modelBundle?.compiledModel && !modelBundle.compiledModel.deleted) modelBundle.compiledModel.delete();
        modelBundle = null;
        if (runtimeLoaded) litert.unloadLiteRt();
        runtimeLoaded = false;
    };
    return {
        async compile(assetsBase) {
            if (runtimeLoaded || modelBundle) throw new Error('Local AI model is already loaded');
            const started = now();
            try {
                const bundle = await loadLocalAiUpscalingBundle(assetsBase, fetchAsset);
                const options = await localAiExecutionOptions(litert, { webgpu, crossOriginIsolated, hardwareConcurrency });
                // LiteRT's wasm-utils forwards self.Module to Emscripten. In a
                // worker the glue otherwise resolves Wasm beside the worker
                // entry, not beside the imported glue. Pin only bundled files.
                const wasmModule = localAiWasmModule(assetsBase, options.loadOptions.threads);
                moduleScope.Module = wasmModule;
                try {
                    await litert.loadLiteRt(new URL('wasm/', assetsBase).href, options.loadOptions);
                } finally {
                    if (moduleScope.Module === wasmModule) delete moduleScope.Module;
                }
                runtimeLoaded = true;
                // LiteRT 2.5.2 non-JSPI glue has no Asyncify for GPU pixel readback.
                const { gpuReadback } = options;
                const compiledModel = await litert.loadAndCompile(new URL(`models/${bundle.modelFile.path}`, assetsBase).href, options.compileOptions);
                modelBundle = { ...bundle, compiledModel };
                const accelerator = webgpu ? (gpuReadback && compiledModel.isFullyAccelerated ? 'webgpu' : 'wasm_fallback') : 'wasm';
                return {
                    upscalingModel: bundle.upscalingModel,
                    models: bundle.models.length,
                    outputTileSize: bundle.outputTileSize,
                    accelerator,
                    webgpu,
                    compile_ms: Math.round(now() - started),
                    execution: options.execution,
                };
            } catch (error) {
                release();
                throw error;
            }
        },
        async tile(inputBytes) {
            if (!modelBundle) throw new Error('Local AI model is not ready');
            if (!(inputBytes instanceof Uint8Array) || inputBytes.length !== LOCAL_AI_UPSCALE_TILE_SIZE ** 2 * 3) {
                throw new Error('Local AI input tile is invalid');
            }
            const started = now();
            const outputData = await runLocalAiUpscaleTile(modelBundle.compiledModel, litert.Tensor, inputBytes, modelBundle.inputShape);
            return { outputData, inference_ms: Math.round(now() - started) };
        },
        release,
    };
}
