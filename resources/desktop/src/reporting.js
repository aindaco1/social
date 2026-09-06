export function observedNumber(value) {
    if (typeof value !== 'number' && typeof value !== 'string') return null;
    if (typeof value === 'string' && !value.trim()) return null;
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
}

export function formatObservation(value) {
    const number = observedNumber(value);
    return number === null ? 'No data' : new Intl.NumberFormat().format(number);
}
