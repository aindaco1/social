import assert from 'node:assert/strict';
import { test } from 'node:test';
import { createLocalMediaOperation, localMediaStatus } from '../resources/desktop/src/localMediaOperation.js';

function deferred() {
    let resolve;
    const promise = new Promise((done) => { resolve = done; });
    return { promise, resolve };
}

function fixture() {
    let state;
    const operation = createLocalMediaOperation((value) => { state = value; });
    return { operation, get state() { return state; } };
}

test('local media operations serialize work and do not offer cancellation for native atomic actions', async () => {
    const f = fixture();
    const pending = deferred();
    const run = f.operation.run('Crop', () => pending.promise);
    assert.equal(f.state.busy, true);
    assert.equal(f.state.canCancel, false);
    assert.equal(f.operation.cancel(), false);
    assert.equal(await f.operation.run('Probe', () => assert.fail('A second operation started')), false);
    pending.resolve({ derivative: { name: 'crop.png' } });
    assert.equal(await run, true);
    assert.equal(f.state.result.status, 'complete');
    assert.equal(f.state.busy, false);
});

test('canceling a model step stays busy until it finishes and blocks derivative commit', async () => {
    const f = fixture();
    const tile = deferred();
    let saves = 0;
    const run = f.operation.run('Upscale', async ({ setProgress, beginCommit }) => {
        await tile.promise;
        setProgress('Finished tile');
        assert.equal(f.state.result.status, 'canceling');
        beginCommit();
        saves += 1;
    }, { cancellable: true, afterSuccess: () => assert.fail('Canceled work refreshed as success') });
    assert.equal(f.operation.cancel(), true);
    assert.equal(f.operation.cancel(), false);
    assert.equal(f.state.busy, true);
    assert.equal(f.state.canCancel, false);
    tile.resolve();
    assert.equal(await run, false);
    assert.equal(saves, 0);
    assert.equal(f.state.result.status, 'canceled');
    assert.match(localMediaStatus(f.state.result), /No output was saved/);
    assert.equal(f.state.busy, false);
});

test('once saving begins, late cancellation cannot mislabel a persisted derivative', async () => {
    const f = fixture();
    const save = deferred();
    const run = f.operation.run('Upscale', async ({ beginCommit }) => {
        beginCommit();
        return save.promise;
    }, { cancellable: true });
    assert.equal(f.state.canCancel, false);
    assert.equal(f.operation.cancel(), false);
    assert.match(localMediaStatus(f.state.result), /Saving derivative/);
    save.resolve({ derivative: { name: 'upscaled.png' } });
    assert.equal(await run, true);
    assert.match(localMediaStatus(f.state.result), /complete · upscaled.png/);
});

test('failure clears busy state and a later operation can succeed without stale errors', async () => {
    const f = fixture();
    assert.equal(await f.operation.run('Search', () => { throw new Error('Missing file'); }), false);
    assert.match(f.state.error, /Missing file/);
    assert.equal(f.state.busy, false);
    assert.equal(f.state.result, null);
    assert.equal(await f.operation.run('Preflight', () => ({ warnings: [] })), true);
    assert.equal(f.state.error, '');
    assert.equal(f.state.result.status, 'complete');
});

test('refresh failure preserves completed output and warns against repeating the operation', async () => {
    const f = fixture();
    assert.equal(await f.operation.run('Crop', () => ({ derivative: { name: 'crop.png' } }), {
        afterSuccess: () => { throw new Error('Refresh unavailable'); },
    }), true);
    assert.equal(f.state.result.status, 'complete');
    assert.match(f.state.error, /completed, but the media list could not refresh/);
    assert.match(f.state.error, /Reopen Media before retrying/);
});

test('local media feedback distinguishes empty searches, limited results, and generated drafts', () => {
    const complete = (result) => ({ operation: 'Search', status: 'complete', result });
    assert.equal(localMediaStatus(null), '');
    assert.match(localMediaStatus(complete({ query: 'missing', matches: [] })), /No local media matched “missing”/);
    assert.match(localMediaStatus(complete({ query: 'blue', matches: Array(12).fill({}) })), /Showing 6 of 12 returned matches/);
    assert.match(localMediaStatus(complete({ query: '', matches: [{}] })), /Showing 1 of 1/);
    assert.match(localMediaStatus(complete({ media: { name: 'image.png' }, alt_text: 'Image properties' })), /Generated alt-text draft for image.png. Review and edit/);
});
