import { readonly, ref } from 'vue';

export const APPEARANCE_STORAGE_KEY = 'dust-wave-social-appearance';
export const appearanceOptions = Object.freeze([
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
]);

const normalizePreference = (value) => (
    appearanceOptions.some((option) => option.value === value) ? value : 'system'
);

// One owner for persistence, OS changes, and the resolved theme used by CSS and canvas UI.
export function createAppearance({ storage, mediaQuery, root } = {}) {
    const preference = ref('system');
    const resolvedTheme = ref('light');
    const persistenceError = ref('');

    try {
        preference.value = normalizePreference(storage?.getItem(APPEARANCE_STORAGE_KEY));
    } catch {
        // Appearance remains usable if local storage is unavailable.
    }

    const apply = () => {
        const theme = preference.value === 'system'
            ? (mediaQuery?.matches ? 'dark' : 'light')
            : preference.value;
        root?.setAttribute('data-theme', theme);
        resolvedTheme.value = theme;
    };

    const setPreference = (value) => {
        preference.value = normalizePreference(value);
        persistenceError.value = '';
        try {
            storage?.setItem(APPEARANCE_STORAGE_KEY, preference.value);
        } catch {
            persistenceError.value = 'Appearance changed for this session, but could not be saved on this Mac.';
        }
        apply();
    };

    mediaQuery?.addEventListener('change', apply);
    apply();

    return {
        preference: readonly(preference),
        resolvedTheme: readonly(resolvedTheme),
        persistenceError: readonly(persistenceError),
        setPreference,
        dispose: () => mediaQuery?.removeEventListener('change', apply),
    };
}

let storage;
try {
    storage = typeof window === 'undefined' ? undefined : window.localStorage;
} catch { /* Use the system setting when storage access is blocked. */ }

export const appearance = createAppearance({
    storage,
    mediaQuery: typeof window === 'undefined' ? undefined : window.matchMedia?.('(prefers-color-scheme: dark)'),
    root: typeof document === 'undefined' ? undefined : document.documentElement,
});

if (import.meta.hot) {
    import.meta.hot.dispose(appearance.dispose);
}
