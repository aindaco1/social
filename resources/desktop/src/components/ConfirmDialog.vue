<script setup>
import { nextTick, ref, watch } from 'vue';

const props = defineProps({
    open: {
        type: Boolean,
        default: false,
    },
    title: {
        type: String,
        required: true,
    },
    description: {
        type: String,
        required: true,
    },
    confirmLabel: {
        type: String,
        default: 'Continue',
    },
    busy: {
        type: Boolean,
        default: false,
    },
    danger: {
        type: Boolean,
        default: false,
    },
});

const emit = defineEmits(['cancel', 'confirm']);
const dialog = ref(null);

watch(() => props.open, async (open) => {
    if (!open) {
        return;
    }

    await nextTick();
    dialog.value?.focus();
});
</script>

<template>
    <div
        v-if="open"
        class="modal-backdrop"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirmation-dialog-title"
        aria-describedby="confirmation-dialog-description"
        @click.self="emit('cancel')"
    >
        <div
            ref="dialog"
            class="confirmation-dialog"
            tabindex="-1"
            @keydown.esc.prevent="emit('cancel')"
        >
            <header>
                <div>
                    <h3 id="confirmation-dialog-title">{{ title }}</h3>
                    <small>Review the effect before continuing.</small>
                </div>
                <button
                    type="button"
                    class="modal-close-button"
                    aria-label="Cancel confirmation"
                    :disabled="busy"
                    @click="emit('cancel')"
                >
                    &times;
                </button>
            </header>
            <p id="confirmation-dialog-description">{{ description }}</p>
            <div class="modal-actions">
                <button type="button" class="inline-button" :disabled="busy" @click="emit('cancel')">
                    Cancel
                </button>
                <button
                    type="button"
                    :class="{ 'danger-button': danger }"
                    :disabled="busy"
                    @click="emit('confirm')"
                >
                    {{ busy ? 'Working…' : confirmLabel }}
                </button>
            </div>
        </div>
    </div>
</template>
