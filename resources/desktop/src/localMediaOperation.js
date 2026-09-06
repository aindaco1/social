export function throwIfLocalMediaCanceled(signal) {
    if (!signal?.aborted) return;
    const error = new Error('Local media operation canceled');
    error.name = 'AbortError';
    throw error;
}

// One operation owns progress and cancellation. Saving is an explicit, non-cancelable boundary.
export function createLocalMediaOperation(onChange) {
    let active = null;
    let state = { busy: false, canCancel: false, result: null, error: '' };
    const update = (changes) => {
        state = { ...state, ...changes };
        onChange(state);
    };
    update({});

    return {
        cancel() {
            if (!active || !state.canCancel) return false;
            active.abort();
            update({
                canCancel: false,
                result: { ...state.result, status: 'canceling', detail: 'Stopping local processing' },
            });
            return true;
        },
        async run(operation, action, { cancellable = false, afterSuccess } = {}) {
            if (active) return false;
            const controller = new AbortController();
            active = controller;
            update({ busy: true, canCancel: cancellable, error: '', result: { operation, status: 'running', detail: 'Starting' } });
            try {
                const result = await action({
                    signal: controller.signal,
                    setProgress(detail) {
                        if (!controller.signal.aborted) update({ result: { operation, status: 'running', detail } });
                    },
                    beginCommit() {
                        throwIfLocalMediaCanceled(controller.signal);
                        update({ canCancel: false, result: { operation, status: 'running', detail: 'Saving derivative' } });
                    },
                });
                throwIfLocalMediaCanceled(controller.signal);
                update({ canCancel: false, result: { operation, status: 'complete', result } });
                try {
                    await afterSuccess?.(result);
                } catch (error) {
                    // The output already exists; a refresh failure must not invite a duplicate operation.
                    update({ error: `${operation} completed, but the media list could not refresh. Reopen Media before retrying. ${String(error)}` });
                }
                return true;
            } catch (error) {
                if (controller.signal.aborted) {
                    update({ result: { operation, status: 'canceled', detail: 'Canceled. No output was saved.' } });
                } else {
                    update({ result: null, error: String(error) });
                }
                return false;
            } finally {
                active = null;
                update({ busy: false, canCancel: false });
            }
        },
    };
}

export const LOCAL_MEDIA_SEARCH_PREVIEW_LIMIT = 6;

export function localMediaStatus(result) {
    if (!result) return '';
    const { operation, status, detail, result: output } = result;
    if (status !== 'complete') return `${operation}: ${detail}`;
    if (Array.isArray(output?.matches)) {
        const query = output.query ? ` for “${output.query}”` : '';
        return output.matches.length
            ? `Showing ${Math.min(LOCAL_MEDIA_SEARCH_PREVIEW_LIMIT, output.matches.length)} of ${output.matches.length} returned matches${query}.`
            : `No local media matched${output.query ? ` “${output.query}”` : ''}. Try a filename, orientation, or color.`;
    }
    if (output?.alt_text) return `Generated alt-text draft for ${output.media.name}. Review and edit it below.`;
    return `${operation} complete${output?.derivative ? ` · ${output.derivative.name}` : ''}`;
}
