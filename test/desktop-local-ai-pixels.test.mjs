import assert from 'node:assert/strict';
import { test } from 'node:test';
import { localAiOutputRgba } from '../resources/desktop/src/localAiPixels.js';

test('quantized model bytes preserve dark values instead of turning 1 into white', () => {
    assert.deepEqual([...localAiOutputRgba(new Uint8Array([0, 1, 2, 128, 254, 255]))],
        [0, 1, 2, 255, 128, 254, 255, 255]);
});

test('normalized floating-point model pixels scale by their tensor type', () => {
    assert.deepEqual([...localAiOutputRgba(new Float32Array([0, 0.5, 1, -1, 2, 0]))],
        [0, 128, 255, 255, 0, 255, 0, 255]);
});

test('unsupported, malformed, and nonfinite model pixels cannot be saved', () => {
    for (const value of [new Int8Array(3), new Uint8Array(2), new Uint8Array(), new Float32Array([0, NaN, 1])]) {
        assert.throws(() => localAiOutputRgba(value));
    }
});
