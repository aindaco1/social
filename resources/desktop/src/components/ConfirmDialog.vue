<script setup>
import { vDialogFocus } from '../dialogFocus.js';

defineProps({
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
    cancelLabel: { type: String, default: 'Cancel' },
    secondaryLabel: { type: String, default: '' },
    busy: {
        type: Boolean,
        default: false,
    },
    danger: {
        type: Boolean,
        default: false,
    },
});

const emit = defineEmits(['cancel', 'confirm', 'secondary']);
</script>

<template>
    <div
        v-if="open"
        v-dialog-focus
        class="modal-backdrop"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirmation-dialog-title"
        aria-describedby="confirmation-dialog-description"
        @click.self="!busy && emit('cancel')"
        @keydown.esc.stop.prevent="!busy && emit('cancel')"
    >
        <div class="confirmation-dialog">
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
                    {{ cancelLabel }}
                </button>
                <button v-if="secondaryLabel" type="button" class="inline-button" :disabled="busy" @click="emit('secondary')">{{ secondaryLabel }}</button>
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
