import test from 'node:test';
import assert from 'node:assert/strict';
import { createDraftReplacementGuard } from '../resources/desktop/src/draftProtection.js';

function fixture({ dirty = true, choice = false, saved = true } = {}) {
    const calls = [];
    const guard = createDraftReplacementGuard({
        hasChanges: () => dirty,
        choose: async () => { calls.push('choose'); return choice; },
        save: async () => { calls.push('save'); return saved; },
        onError: (error) => calls.push(error.message),
    });
    return { calls, run: () => guard(() => calls.push('replace')) };
}

test('a save or schedule already in flight blocks unrelated replacement', async () => {
    let replaced = false;
    const guard = createDraftReplacementGuard({
        isBusy: () => true, hasChanges: () => false,
        choose: async () => true, save: async () => true,
        onError: assert.fail,
    });
    assert.equal(await guard(() => { replaced = true; }), false);
    assert.equal(replaced, false);
});

test('replacement keeps unsaved work by default and asks only when dirty', async () => {
    const keep = fixture();
    assert.equal(await keep.run(), false);
    assert.deepEqual(keep.calls, ['choose']);
    const clean = fixture({ dirty: false });
    assert.equal(await clean.run(), true);
    assert.deepEqual(clean.calls, ['replace']);
});

test('save must succeed before replacing, and explicit discard skips saving', async () => {
    for (const [options, expected] of [
        [{ choice: 'save' }, ['choose', 'save', 'replace']],
        [{ choice: 'save', saved: false }, ['choose', 'save']],
        [{ choice: true }, ['choose', 'replace']],
    ]) {
        const f = fixture(options);
        await f.run();
        assert.deepEqual(f.calls, expected);
    }
});

test('rapid competing replacements cannot bypass the pending decision and errors unlock retry', async () => {
    let decide;
    const calls = [];
    const guard = createDraftReplacementGuard({
        hasChanges: () => true,
        choose: () => new Promise((resolve) => { decide = resolve; }),
        save: async () => { throw new Error('Save failed'); },
        onError: (error) => calls.push(error.message),
    });
    const pending = guard(() => calls.push('replace'));
    assert.equal(await guard(() => calls.push('bypass')), false);
    decide('save');
    assert.equal(await pending, false);
    assert.deepEqual(calls, ['Save failed']);
    const retry = guard(() => calls.push('replace'));
    decide(true);
    assert.equal(await retry, true);
    assert.deepEqual(calls, ['Save failed', 'replace']);
});
