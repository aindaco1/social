// One replacement boundary for calendar shortcuts, explicit clearing, and opening another post.
export function createDraftReplacementGuard({ hasChanges, choose, save, onError, isBusy = () => false }) {
    let pending = false;
    return async (replace) => {
        if (pending || isBusy()) return false;
        pending = true;
        try {
            if (hasChanges()) {
                const choice = await choose();
                if (!choice) return false;
                if (choice === 'save' && !await save()) return false;
            }
            await replace();
            return true;
        } catch (error) {
            onError(error);
            return false;
        } finally {
            pending = false;
        }
    };
}
