<script setup>
import { nextTick, onBeforeMount, onMounted, onUnmounted, ref } from 'vue';

const props = defineProps({
    title: {
        type: String,
        required: true,
    },
    description: {
        type: String,
        default: '',
    },
    saveLabel: {
        type: String,
        default: 'Save',
    },
    busy: {
        type: Boolean,
        default: false,
    },
});

const emit = defineEmits(['save', 'cancel']);
const editor = ref(null);
let returnFocusTarget = null;

const save = () => {
    if (!props.busy) {
        emit('save');
    }
};

const cancel = () => {
    if (!props.busy) {
        emit('cancel');
    }
};

onBeforeMount(() => {
    returnFocusTarget = document.activeElement;
});

onMounted(async () => {
    await nextTick();
    editor.value
        ?.querySelector('[data-contextual-autofocus], input:not(:disabled), textarea:not(:disabled), select:not(:disabled)')
        ?.focus();
});

onUnmounted(() => {
    if (returnFocusTarget instanceof HTMLElement && returnFocusTarget.isConnected) {
        returnFocusTarget.focus();
    }
});
</script>

<template>
    <form
        ref="editor"
        class="contextual-editor"
        :aria-label="title"
        :aria-busy="busy"
        @submit.prevent="save"
        @keydown.esc.stop.prevent="cancel"
    >
        <header class="contextual-editor-header">
            <div>
                <strong>{{ title }}</strong>
                <small v-if="description">{{ description }}</small>
            </div>
        </header>
        <div class="contextual-editor-fields">
            <slot></slot>
        </div>
        <div class="contextual-editor-actions">
            <button type="button" class="inline-button" :disabled="busy" @click="cancel">
                Cancel
            </button>
            <button type="submit" :disabled="busy">
                {{ busy ? 'Saving…' : saveLabel }}
            </button>
        </div>
    </form>
</template>
