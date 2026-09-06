import { throwIfLocalMediaCanceled } from './localMediaOperation.js';

// A fresh worker owns one operation's model and Wasm heap. Termination interrupts
// synchronous CPU inference; a cancel message alone cannot interrupt that work.
export function createLocalAiWorkerClient({ createWorker, signal }) {
    throwIfLocalMediaCanceled(signal);
    const worker = createWorker();
    let pending = null;
    let sequence = 0;
    let closed = false;
    const dispose = (error = new Error('Local AI worker closed')) => {
        if (closed) return;
        closed = true;
        signal?.removeEventListener('abort', abort);
        worker.onmessage = worker.onerror = worker.onmessageerror = null;
        worker.terminate();
        pending?.reject(error);
        pending = null;
    };
    const abort = () => {
        try { throwIfLocalMediaCanceled(signal); } catch (error) { dispose(error); }
    };
    signal?.addEventListener('abort', abort, { once: true });
    worker.onerror = (event) => {
        event.preventDefault?.();
        dispose(new Error(event.message || 'Local AI worker could not start'));
    };
    worker.onmessageerror = () => dispose(new Error('Local AI worker returned unreadable data'));
    worker.onmessage = ({ data }) => {
        if (!pending || data?.id !== pending.id) return;
        if (typeof data.ok !== 'boolean') {
            dispose(new Error('Local AI worker returned an invalid response'));
            return;
        }
        if (!data.ok) {
            dispose(new Error(data.error || 'Local AI worker failed'));
            return;
        }
        const current = pending;
        pending = null;
        current.resolve(data.result);
    };
    return {
        request(type, payload = {}, transfer = []) {
            try { throwIfLocalMediaCanceled(signal); } catch (error) { return Promise.reject(error); }
            if (closed) return Promise.reject(new Error('Local AI worker closed'));
            if (pending) return Promise.reject(new Error('Local AI worker is already processing'));
            return new Promise((resolve, reject) => {
                pending = { id: ++sequence, resolve, reject };
                try { worker.postMessage({ id: sequence, type, payload }, transfer); }
                catch (error) { dispose(error); }
            });
        },
        dispose,
    };
}

// Session-only diagnostics: measure the UI event loop, not native-control latency.
// Hidden/background windows can throttle timers; this is not a universal benchmark.
export function observeLocalAiResponsiveness({ now = () => performance.now(), schedule = setInterval, unschedule = clearInterval } = {}) {
    const started = now();
    let previous = started;
    let maxGapMs = 0;
    let samples = 0;
    const sample = () => {
        const current = now();
        maxGapMs = Math.max(maxGapMs, current - previous);
        previous = current;
        samples += 1;
    };
    const timer = schedule(sample, 100);
    let result;
    return () => {
        if (result) return result;
        unschedule(timer);
        sample();
        result = { elapsed_ms: Math.round(now() - started), ui_max_gap_ms: Math.round(maxGapMs), ui_samples: samples };
        return result;
    };
}
