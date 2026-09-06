// Instants stay UTC in storage. Date-only provider observations remain dates, not instants.
const formatters = new Map();
function partsFormatter(timeZone) {
    if (!formatters.has(timeZone)) formatters.set(timeZone, new Intl.DateTimeFormat('en-CA', {
        timeZone, year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23',
    }));
    return formatters.get(timeZone);
}

export function instant(value) {
    if (value === null || value === undefined || value === '') return null;
    // SQLite CURRENT_TIMESTAMP has no suffix but is UTC, never the Mac's local time.
    const text = typeof value === 'string' ? value.replace(' ', 'T') : value;
    const normalized = typeof text === 'string' && /^\d{4}-\d\d-\d\dT\d\d:\d\d(?::\d\d(?:\.\d+)?)?$/.test(text) ? `${text}Z` : text;
    const date = new Date(normalized);
    return Number.isNaN(date.getTime()) ? null : date;
}

export function localDateTime(value, timeZone = 'UTC') {
    const date = instant(value);
    if (!date) return '';
    const p = Object.fromEntries(partsFormatter(timeZone).formatToParts(date).map(({ type, value }) => [type, value]));
    return `${p.year}-${p.month}-${p.day}T${p.hour}:${p.minute}`;
}

export function parseSchedule(value, timeZone = 'UTC', originalInstant = null) {
    if (!value) return { date: null, error: '' };
    const original = instant(originalInstant);
    if (!/^\d{4}-\d\d-\d\dT\d\d:\d\d$/.test(value)) return { date: null, error: 'Enter a valid date and time.' };
    const wall = new Date(`${value}:00Z`).getTime();
    if (!Number.isFinite(wall)) return { date: null, error: 'Enter a valid date and time.' };
    // Sample offsets on both sides of the date; validate each candidate by a round trip.
    // This catches missing/repeated hours and half-hour DST without silently choosing an instant.
    const offsets = new Set();
    try {
        if (original && localDateTime(original, timeZone) === value) return { date: original, error: '' };
        for (let hours = -48; hours <= 48; hours += 6) {
            const sample = wall + hours * 3600000;
            offsets.add(new Date(`${localDateTime(sample, timeZone)}:00Z`).getTime() - sample);
        }
        const matches = [...offsets].map((offset) => new Date(wall - offset))
            .filter((date) => localDateTime(date, timeZone) === value);
        if (matches.length === 1) return { date: matches[0], error: '' };
        return { date: null, error: matches.length
            ? `This time occurs twice in ${timeZone} because the clocks go back. Choose another time, or schedule in UTC.`
            : `This time does not exist in ${timeZone}. Check the date and daylight-saving change.` };
    } catch {
        return { date: null, error: 'Choose a supported timezone in Settings.' };
    }
}

export function formatDateOnly(value, dateFormat = 'human') {
    if (!value) return 'No data';
    if (dateFormat === 'iso') return value.slice(0, 10);
    const date = instant(`${value.slice(0, 10)}T12:00:00Z`);
    return date ? new Intl.DateTimeFormat(undefined, { timeZone: 'UTC', year: 'numeric', month: 'short', day: 'numeric' }).format(date) : value;
}

export function formatTimestamp(value, settings = {}, { timeOnly = false } = {}) {
    const date = instant(value);
    if (!date) return '—';
    const timeZone = settings.timezone || 'UTC';
    const time = new Intl.DateTimeFormat(undefined, { timeZone, hour: 'numeric', minute: '2-digit', hour12: Number(settings.time_format || 12) === 12 }).format(date);
    return timeOnly ? time : `${formatDateOnly(localDateTime(date, timeZone), settings.date_format)} · ${time}`;
}

export function suggestedSchedule(date, timeZone, hour = 9, now = new Date()) {
    const desired = `${date}T${String(hour).padStart(2, '0')}:00`;
    const parsed = parseSchedule(desired, timeZone);
    if (parsed.error) return desired;
    if (parsed.date && parsed.date > now) return desired;
    return localDateTime(new Date(Math.ceil((now.getTime() + 30 * 60000) / (15 * 60000)) * 15 * 60000), timeZone);
}

export function calendarDates(window) {
    if (!window?.start_date || !window?.end_date) return [];
    const end = Date.parse(`${window.end_date}T12:00:00Z`);
    const dates = [];
    for (let date = Date.parse(`${window.start_date}T12:00:00Z`); date <= end && dates.length < 42; date += 86400000) {
        dates.push(new Date(date).toISOString().slice(0, 10));
    }
    return dates;
}
