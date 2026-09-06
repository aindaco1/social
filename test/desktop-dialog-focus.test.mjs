import assert from 'node:assert/strict';
import { test } from 'node:test';
import { vDialogFocus } from '../resources/desktop/src/dialogFocus.js';

function fixture() {
    const document = new EventTarget();
    const dialog = new EventTarget();
    const control = (options = {}) => ({
        disabled: false, tabIndex: 0, isConnected: true,
        getClientRects: () => [1],
        focus() { document.activeElement = this; },
        ...options,
    });
    const opener = control();
    const first = control();
    const last = control();
    let top = dialog;
    let items = [first, control({ disabled: true }), control({ getClientRects: () => [] }), last];
    Object.assign(dialog, {
        ownerDocument: document,
        querySelectorAll: () => items,
        contains: (target) => target === dialog || items.includes(target),
        focus() { document.activeElement = this; },
    });
    document.activeElement = opener;
    document.querySelectorAll = () => [top];
    const tab = (shiftKey = false) => {
        const event = new Event('keydown', { cancelable: true });
        Object.assign(event, { key: 'Tab', shiftKey });
        dialog.dispatchEvent(event);
        return event;
    };
    return { document, dialog, opener, first, last, tab, setTop: (value) => { top = value; }, empty: () => { items = []; } };
}

test('dialogs autofocus, wrap Tab in both directions, and restore the opener', () => {
    const f = fixture();
    vDialogFocus.mounted(f.dialog);
    assert.equal(f.document.activeElement, f.first);
    assert.equal(f.tab(true).defaultPrevented, true);
    assert.equal(f.document.activeElement, f.last);
    assert.equal(f.tab().defaultPrevented, true);
    assert.equal(f.document.activeElement, f.first);
    assert.equal(f.tab().defaultPrevented, false);
    vDialogFocus.unmounted(f.dialog);
    assert.equal(f.document.activeElement, f.opener);
    assert.equal(f.tab().defaultPrevented, false);
});

test('only the top dialog traps focus; an empty dialog remains keyboard-safe', () => {
    const f = fixture();
    vDialogFocus.mounted(f.dialog);
    f.setTop({});
    assert.equal(f.tab(true).defaultPrevented, false);
    f.setTop(f.dialog);
    f.empty();
    assert.equal(f.tab().defaultPrevented, true);
    assert.equal(f.document.activeElement, f.dialog);
    f.opener.isConnected = false;
    vDialogFocus.unmounted(f.dialog);
    assert.equal(f.document.activeElement, f.dialog);
});

test('changing dialog steps restores lost focus without stealing it from fields or nested dialogs', () => {
    const f = fixture();
    vDialogFocus.mounted(f.dialog);
    f.document.activeElement = f.opener; // Browser falls back outside when a step removes its button.
    vDialogFocus.updated(f.dialog);
    assert.equal(f.document.activeElement, f.first);
    f.last.focus();
    vDialogFocus.updated(f.dialog);
    assert.equal(f.document.activeElement, f.last);
    f.setTop({});
    f.document.activeElement = f.opener;
    vDialogFocus.updated(f.dialog);
    assert.equal(f.document.activeElement, f.opener);
    vDialogFocus.unmounted(f.dialog);
});
