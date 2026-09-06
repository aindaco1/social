import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';
import postcss from 'postcss';
import { APPEARANCE_STORAGE_KEY, createAppearance } from '../resources/desktop/src/appearance.js';

function environment({ saved = null, dark = false } = {}) {
    const values = new Map(saved ? [[APPEARANCE_STORAGE_KEY, saved]] : []);
    const listeners = new Set();
    const attributes = new Map();
    const storage = { getItem: (key) => values.get(key), setItem: (key, value) => values.set(key, value) };
    const mediaQuery = {
        matches: dark,
        addEventListener: (_event, fn) => listeners.add(fn),
        removeEventListener: (_event, fn) => listeners.delete(fn),
    };
    return {
        storage, mediaQuery, values, listeners, attributes,
        root: { setAttribute: (key, value) => attributes.set(key, value) },
        changeSystem(next) {
            mediaQuery.matches = next;
            listeners.forEach((fn) => fn());
        },
    };
}

test('appearance follows the system by default and responds to OS changes', () => {
    const env = environment({ dark: true });
    const state = createAppearance(env);
    assert.equal(state.preference.value, 'system');
    assert.equal(env.attributes.get('data-theme'), 'dark');
    env.changeSystem(false);
    assert.equal(state.resolvedTheme.value, 'light');
    assert.equal(env.attributes.get('data-theme'), 'light');
    state.dispose();
    assert.equal(env.listeners.size, 0);
});

test('manual appearance persists across launch and overrides subsequent OS changes', () => {
    const env = environment({ dark: true });
    const state = createAppearance(env);
    state.setPreference('light');
    env.changeSystem(false);
    env.changeSystem(true);
    assert.equal(state.resolvedTheme.value, 'light');
    assert.equal(env.values.get(APPEARANCE_STORAGE_KEY), 'light');
    state.dispose();
    const relaunched = createAppearance(env);
    assert.equal(relaunched.resolvedTheme.value, 'light');
    relaunched.setPreference('dark');
    env.changeSystem(false);
    assert.equal(relaunched.resolvedTheme.value, 'dark');
    relaunched.setPreference('system');
    assert.equal(relaunched.resolvedTheme.value, 'light');
    relaunched.dispose();
});

test('invalid saved preferences recover to the system and blocked storage does not break appearance', () => {
    const invalid = createAppearance(environment({ saved: 'sepia', dark: true }));
    assert.equal(invalid.preference.value, 'system');
    assert.equal(invalid.resolvedTheme.value, 'dark');
    invalid.dispose();
    const env = environment();
    env.storage = { getItem() { throw new Error('blocked'); }, setItem() { throw new Error('full'); } };
    const state = createAppearance(env);
    state.setPreference('dark');
    assert.equal(state.resolvedTheme.value, 'dark');
    assert.match(state.persistenceError.value, /could not be saved/);
    state.dispose();
});

const styles = postcss.parse(await readFile(new URL('../resources/desktop/src/styles.css', import.meta.url), 'utf8'));
const light = {};
const dark = {};
styles.walkRules((rule) => {
    const target = rule.selector === ':root' ? light : rule.selector === ':root[data-theme="dark"]' ? dark : null;
    if (target) rule.walkDecls((decl) => { target[decl.prop] = decl.value; });
});

function luminance(hex) {
    const rgb = hex.slice(1).match(/.{2}/g).map((part) => parseInt(part, 16) / 255)
        .map((c) => c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
    return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
}

for (const [name, theme] of Object.entries({ light, dark: { ...light, ...dark } })) {
    test(`${name} theme keeps normal text and semantic status text legible`, () => {
        const pairs = ['surface-page', 'surface-base', 'surface-subtle'].flatMap((surface) => (
            ['ink-strong', 'ink', 'ink-muted', 'ink-soft'].map((ink) => [ink, surface])
        ));
        pairs.push(['success', 'success-bg'], ['warning', 'warning-bg'], ['danger-dark', 'danger-bg'], ['link', 'surface-base']);
        for (const [foreground, background] of pairs) {
            const a = luminance(theme[`--dw-${foreground}`]);
            const b = luminance(theme[`--dw-${background}`]);
            const ratio = (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
            assert.ok(ratio >= 4.5, `${name} ${foreground} on ${background}: ${ratio.toFixed(2)}:1`);
        }
    });
}

test('every semantic color reference resolves and the dark palette covers the light surfaces', () => {
    styles.walkDecls((decl) => {
        for (const [, variable] of decl.value.matchAll(/var\((--dw-[\w-]+)/g)) {
            assert.ok(variable in light, `Missing token ${variable}`);
        }
    });
    for (const key of Object.keys(light).filter((key) => key.startsWith('--dw-surface-'))) {
        assert.ok(key in dark, `Missing dark surface ${key}`);
    }
});
