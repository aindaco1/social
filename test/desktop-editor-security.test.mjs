import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { getSchema } from '@tiptap/core';
import Document from '@tiptap/extension-document';
import Text from '@tiptap/extension-text';
import Link from '@tiptap/extension-link';
import Div from '../resources/js/Extensions/TipTap/Div.js';

test('the fixed composer schema discards JSON-origin prototype and event attributes', () => {
    const schema = getSchema([Document, Div, Text, Link.configure({ openOnClick: false, linkOnPaste: false })]);
    const attributes = JSON.parse('{"__proto__":{"onerror":"canary","data-canary":"inherited"},"onerror":"canary","href":"https://example.com/"}');
    const node = schema.nodeFromJSON({
        type: 'doc', content: [{ type: 'div', attrs: attributes, content: [
            { type: 'text', text: 'Example', marks: [{ type: 'link', attrs: attributes }] },
        ] }],
    }).firstChild;
    const mark = node.firstChild.marks[0];
    for (const rendered of [node.type.spec.toDOM(node), mark.type.spec.toDOM(mark, true)]) {
        const renderedAttributes = rendered[1];
        assert.equal(Object.getPrototypeOf(renderedAttributes), Object.prototype);
        assert.equal(Object.hasOwn(renderedAttributes, '__proto__'), false);
        assert.equal('onerror' in renderedAttributes, false);
        assert.equal('data-canary' in renderedAttributes, false);
    }
});

test('the desktop composer keeps string content and static attributes as security boundaries', () => {
    const app = readFileSync(new URL('../resources/desktop/src/App.vue', import.meta.url), 'utf8');
    const div = readFileSync(new URL('../resources/js/Extensions/TipTap/Div.js', import.meta.url), 'utf8');
    const config = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
    assert.match(app, /function normalizeEditorContent\(value\) \{\s*const content = String\(value \|\| ''\);/);
    assert.match(app, /content: normalizeEditorContent\(activeDraftBody.value\)/);
    assert.match(app, /const content = normalizeEditorContent\(activeDraftBody.value\);[\s\S]*?editor.commands.setContent\(content, false\)/);
    assert.doesNotMatch(app, /HTMLAttributes\s*:/);
    assert.match(div, /HTMLAttributes: \{\}/);
    assert.doesNotMatch(div, /addAttributes\s*\(/);
    const scripts = config.app.security.csp.split(';').find(rule => rule.trim().startsWith('script-src'));
    assert.doesNotMatch(scripts, /'unsafe-inline'|'unsafe-eval'/);
});
