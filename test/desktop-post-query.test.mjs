import test from 'node:test';
import assert from 'node:assert/strict';
import { paginateItems, queryPostPages } from '../resources/desktop/src/postQuery.js';

test('dense agenda pages keep every post reachable without rendering hundreds of rows at once', () => {
    const items = Array.from({ length: 205 }, (_, n) => n);
    assert.deepEqual(paginateItems(items, 9), { items: [200, 201, 202, 203, 204], page: 9, pages: 9, total: 205, start: 201, end: 205 });
    assert.equal(paginateItems(items, -2).page, 1);
    assert.equal(paginateItems(items, 20).page, 9);
    assert.equal(paginateItems(items, 1).items.length, 25);
    assert.deepEqual(Array.from({ length: 9 }, (_, n) => paginateItems(items, n + 1).items).flat(), items);
    assert.deepEqual(paginateItems([], 9), { items: [], page: 1, pages: 1, total: 0, start: 0, end: 0 });
});

test('calendar loads every date-window page while library remains paginated', async () => {
    const calls = [];
    const query = async (request) => {
        calls.push(request);
        return { total: 205, total_pages: 2, items: Array.from({ length: request.page === 1 ? 200 : 5 }, (_, n) => (request.page - 1) * 200 + n) };
    };
    const request = { page: 1, limit: 200, date: '2026-09-05', calendar_type: 'month', accounts: [2] };
    assert.equal((await queryPostPages(query, request, { allPages: true })).items.length, 205);
    assert.deepEqual(calls[1], { ...request, page: 2 });
    assert.equal((await queryPostPages(query, request)).items.length, 200);
    assert.equal(calls.length, 3);
});

test('obsolete responses and failed follow-up pages are never committed as a complete calendar', async () => {
    let calls = 0;
    const result = await queryPostPages(async () => { calls += 1; return { total_pages: 4, items: [] }; }, {}, { allPages: true, isCurrent: () => false });
    assert.equal(result, null);
    assert.equal(calls, 1);
    await assert.rejects(queryPostPages(async ({ page }) => {
        if (page === 2) throw new Error('Database unavailable');
        return { total_pages: 2, items: [1] };
    }, { page: 1 }, { allPages: true }), /Database unavailable/);
});
