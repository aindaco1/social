import assert from 'node:assert/strict';
import { test } from 'node:test';
import { vTabNavigation } from '../resources/desktop/src/tabNavigation.js';

function fixture({ vertical = false } = {}) {
    const element = new EventTarget();
    const document = { activeElement: null };
    const items = Array.from({ length: 4 }, (_, index) => ({
        disabled: index === 1,
        selected: index === 2,
        tabIndex: 0,
        closest: () => element,
        getAttribute(name) { return name === 'aria-selected' ? String(this.selected) : null; },
        focus() { document.activeElement = this; },
    }));
    Object.assign(element, {
        ownerDocument: document,
        querySelectorAll: () => items,
        getAttribute: () => vertical ? 'vertical' : null,
    });
    const key = (key, modifiers = {}) => {
        const event = new Event('keydown', { cancelable: true });
        Object.defineProperty(event, 'target', { value: document.activeElement });
        Object.assign(event, { key, ...modifiers });
        element.dispatchEvent(event);
        return event;
    };
    vTabNavigation.mounted(element);
    return { element, document, items, key };
}

test('tablists have one tab stop and arrows skip disabled tabs without activating content', () => {
    const f = fixture();
    assert.deepEqual(f.items.map((tab) => tab.tabIndex), [-1, -1, 0, -1]);
    f.items[2].focus();
    assert.equal(f.key('ArrowLeft').defaultPrevented, true);
    assert.equal(f.document.activeElement, f.items[0]);
    assert.equal(f.items[2].selected, true);
    assert.deepEqual(f.items.map((tab) => tab.tabIndex), [0, -1, -1, -1]);
    f.key('ArrowLeft');
    assert.equal(f.document.activeElement, f.items[3]);
    f.key('ArrowRight');
    assert.equal(f.document.activeElement, f.items[0]);
    f.key('End');
    assert.equal(f.document.activeElement, f.items[3]);
    f.key('Home');
    assert.equal(f.document.activeElement, f.items[0]);
    for (const key of ['Tab', 'Enter', ' ', 'ArrowDown']) assert.equal(f.key(key).defaultPrevented, false);
    assert.equal(f.key('ArrowRight', { metaKey: true }).defaultPrevented, false);
    vTabNavigation.unmounted(f.element);
    assert.equal(f.key('ArrowRight').defaultPrevented, false);
});

test('tab navigation follows updates and vertical orientation, with safe empty lists', () => {
    const f = fixture({ vertical: true });
    f.items[2].focus();
    f.key('ArrowUp');
    assert.equal(f.document.activeElement, f.items[0]);
    f.key('ArrowDown');
    assert.equal(f.document.activeElement, f.items[2]);
    f.document.activeElement = null;
    f.items[2].selected = false;
    f.items[3].selected = true;
    vTabNavigation.updated(f.element);
    assert.deepEqual(f.items.map((tab) => tab.tabIndex), [-1, -1, -1, 0]);
    f.items.forEach((tab) => { tab.disabled = true; });
    vTabNavigation.updated(f.element);
    assert.ok(f.items.every((tab) => tab.tabIndex === -1));
    assert.equal(f.key('ArrowDown').defaultPrevented, false);
    vTabNavigation.unmounted(f.element);
});
