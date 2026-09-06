import test from 'node:test';
import assert from 'node:assert/strict';
import { observedNumber, formatObservation } from '../resources/desktop/src/reporting.js';

test('missing or invalid observations remain absent while measured zero stays zero', () => {
    for (const value of [null, undefined, '', '  ', false, [], 'unknown', NaN, Infinity]) {
        assert.equal(observedNumber(value), null);
        assert.equal(formatObservation(value), 'No data');
    }
    for (const value of [0, '0']) {
        assert.equal(observedNumber(value), 0);
        assert.equal(formatObservation(value), '0');
    }
    assert.equal(observedNumber(12), 12);
});
