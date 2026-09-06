// Shared keyboard behavior for account, post, and confirmation dialogs.
const activeDialogs = new WeakMap();
const focusableSelector = 'button, a[href], input, select, textarea, [tabindex]';

export const vDialogFocus = {
    mounted(element) {
        const document = element.ownerDocument;
        const previousFocus = document.activeElement;
        const isTopDialog = () => Array.from(document.querySelectorAll('[aria-modal="true"]')).at(-1) === element;
        const controls = () => Array.from(element.querySelectorAll(focusableSelector)).filter((control) => (
            !control.disabled && control.tabIndex >= 0 && control.getClientRects().length
        ));
        const focusFirst = () => (controls()[0] || element).focus();
        const onKeydown = (event) => {
            if (event.key !== 'Tab' || !isTopDialog()) return;
            const items = controls();
            const index = items.indexOf(document.activeElement);
            if (!items.length) {
                event.preventDefault();
                element.focus();
            } else if (event.shiftKey && index <= 0) {
                event.preventDefault();
                items.at(-1).focus();
            } else if (!event.shiftKey && (index === -1 || index === items.length - 1)) {
                event.preventDefault();
                items[0].focus();
            }
        };
        const onFocus = (event) => {
            if (isTopDialog() && !element.contains(event.target)) focusFirst();
        };
        const ensureFocus = () => {
            if (isTopDialog() && !element.contains(document.activeElement)) focusFirst();
        };
        element.tabIndex = -1;
        element.addEventListener('keydown', onKeydown);
        document.addEventListener('focusin', onFocus);
        activeDialogs.set(element, {
            ensureFocus,
            cleanup() {
                element.removeEventListener('keydown', onKeydown);
                document.removeEventListener('focusin', onFocus);
                if (previousFocus?.isConnected) previousFocus.focus();
            },
        });
        focusFirst();
    },
    updated(element) {
        // Changing a dialog step can remove its focused button without a focusin event.
        activeDialogs.get(element)?.ensureFocus();
    },
    unmounted(element) {
        activeDialogs.get(element)?.cleanup();
        activeDialogs.delete(element);
    },
};
