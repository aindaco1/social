// The bundled model returns quantized bytes; normalized float tensors use a
// different scale. A dark byte value of 1 must never be mistaken for white.
export function localAiOutputRgba(output) {
    const normalized = output instanceof Float32Array || output instanceof Float64Array;
    if (!normalized && !(output instanceof Uint8Array) && !(output instanceof Uint8ClampedArray)) {
        throw new Error('Local AI model returned an unsupported pixel type');
    }
    if (!output.length || output.length % 3 !== 0) {
        throw new Error('Local AI model returned invalid RGB pixels');
    }
    const rgba = new Uint8ClampedArray(output.length / 3 * 4);
    const scale = normalized ? 255 : 1;
    for (let source = 0, target = 0; source < output.length; source += 3, target += 4) {
        for (let channel = 0; channel < 3; channel += 1) {
            if (!Number.isFinite(output[source + channel])) throw new Error('Local AI model returned invalid RGB pixels');
            rgba[target + channel] = Math.round(output[source + channel] * scale);
        }
        rgba[target + 3] = 255;
    }
    return rgba;
}
