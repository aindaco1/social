import test from 'node:test';
import assert from 'node:assert/strict';
import { calendarDates, formatDateOnly, formatTimestamp, instant, localDateTime, parseSchedule, suggestedSchedule } from '../resources/desktop/src/dateTime.js';

test('workspace wall times round-trip UTC, negative and fractional positive offsets', () => {
    for (const [zone, wall, utc] of [
        ['UTC', '2026-09-06T01:30', '2026-09-06T01:30:00.000Z'],
        ['America/Denver', '2026-09-05T19:30', '2026-09-06T01:30:00.000Z'],
        ['Asia/Kolkata', '2026-09-06T07:00', '2026-09-06T01:30:00.000Z'],
        ['Pacific/Auckland', '2026-09-06T13:30', '2026-09-06T01:30:00.000Z'],
    ]) {
        assert.equal(parseSchedule(wall, zone).date.toISOString(), utc);
        assert.equal(localDateTime(utc, zone), wall);
    }
});

test('DST gaps and repeated hours cannot silently become a different scheduled instant', () => {
    assert.match(parseSchedule('2026-03-08T02:30', 'America/Denver').error, /does not exist/);
    assert.match(parseSchedule('2026-11-01T01:30', 'America/Denver').error, /occurs twice/);
    assert.match(parseSchedule('2026-04-05T01:45', 'Australia/Lord_Howe').error, /occurs twice/);
    for (const existing of ['2026-11-01T07:30:00Z', '2026-11-01T08:30:00Z']) {
        assert.equal(parseSchedule('2026-11-01T01:30', 'America/Denver', existing).date.getTime(), Date.parse(existing));
    }
    assert.equal(parseSchedule('2026-03-08T03:30', 'America/Denver').date.toISOString(), '2026-03-08T09:30:00.000Z');
});

test('invalid dates and zones fail clearly; valid leap days and empty schedules remain supported', () => {
    for (const input of ['2026-02-30T10:00', '2025-02-29T10:00', '2026-09-05T25:00', 'nonsense']) {
        assert.ok(parseSchedule(input, 'UTC').error);
        assert.equal(parseSchedule(input, 'UTC').date, null);
    }
    assert.ok(parseSchedule('2024-02-29T10:00', 'UTC').date);
    assert.match(parseSchedule('2026-09-05T10:00', 'Invalid/Zone', '2026-09-05T10:00:00Z').error, /supported timezone/);
    assert.deepEqual(parseSchedule('', 'UTC'), { date: null, error: '' });
});

test('SQLite timestamps are UTC and display respects workspace date and clock settings', () => {
    assert.equal(instant('2026-09-06 01:30:00').toISOString(), '2026-09-06T01:30:00.000Z');
    assert.equal(instant(0).toISOString(), '1970-01-01T00:00:00.000Z');
    assert.equal(instant(null), null);
    const settings = { timezone: 'America/Denver', date_format: 'iso', time_format: 24 };
    assert.equal(formatTimestamp('2026-09-06 01:30:00', settings), '2026-09-05 · 19:30');
    assert.match(formatTimestamp('2026-09-06T01:30:00Z', { ...settings, time_format: 12 }), /7:30.*PM/);
    assert.equal(formatDateOnly('2026-09-06', 'iso'), '2026-09-06');
    assert.equal(formatTimestamp(null, settings), '—');
});

test('calendar shortcuts suggest a future time and rendered dates consume the backend window exactly', () => {
    const now = new Date('2026-09-05T21:07:00Z');
    assert.equal(suggestedSchedule('2026-09-05', 'America/Denver', 9, now), '2026-09-05T15:45');
    assert.equal(suggestedSchedule('2026-09-08', 'America/Denver', 9, now), '2026-09-08T09:00');
    const dates = calendarDates({ start_date: '2026-08-31', end_date: '2026-10-11' });
    assert.equal(dates.length, 42);
    assert.equal(dates[0], '2026-08-31');
    assert.equal(dates.at(-1), '2026-10-11');
    assert.equal(new Set(dates).size, 42);
    assert.deepEqual(calendarDates(null), []);
});
