// One tab stop per tablist; arrows/Home/End move focus, Enter/Space activate the native button.
// Manual activation avoids starting provider queries while simply moving through choices.
const tabLists = new WeakMap();

export const vTabNavigation = {
    mounted(element) {
        const tabs = () => Array.from(element.querySelectorAll('[role="tab"]')).filter((tab) => (
            tab.closest('[role="tablist"]') === element
            && !tab.disabled && tab.getAttribute('aria-disabled') !== 'true'
        ));
        const sync = () => {
            const items = tabs();
            const active = items.includes(element.ownerDocument.activeElement)
                ? element.ownerDocument.activeElement
                : items.find((tab) => tab.getAttribute('aria-selected') === 'true') || items[0];
            for (const tab of element.querySelectorAll('[role="tab"]')) {
                tab.tabIndex = tab === active ? 0 : -1;
            }
        };
        const onKeydown = (event) => {
            if (event.altKey || event.ctrlKey || event.metaKey) return;
            const items = tabs();
            const index = items.indexOf(event.target);
            if (index === -1) return;
            const vertical = element.getAttribute('aria-orientation') === 'vertical';
            const previous = vertical ? 'ArrowUp' : 'ArrowLeft';
            const next = vertical ? 'ArrowDown' : 'ArrowRight';
            let target;
            if (event.key === 'Home') target = items[0];
            else if (event.key === 'End') target = items.at(-1);
            else if (event.key === previous) target = items[(index - 1 + items.length) % items.length];
            else if (event.key === next) target = items[(index + 1) % items.length];
            else return;
            event.preventDefault();
            target.focus();
            sync();
        };
        element.addEventListener('keydown', onKeydown);
        element.addEventListener('focusin', sync);
        tabLists.set(element, { sync, onKeydown });
        sync();
    },
    updated(element) {
        tabLists.get(element)?.sync();
    },
    unmounted(element) {
        const state = tabLists.get(element);
        if (!state) return;
        element.removeEventListener('keydown', state.onKeydown);
        element.removeEventListener('focusin', state.sync);
        tabLists.delete(element);
    },
};
