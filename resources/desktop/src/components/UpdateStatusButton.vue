<script setup>
import { computed } from 'vue';

const props = defineProps({
    checking: {
        type: Boolean,
        default: false,
    },
    installing: {
        type: Boolean,
        default: false,
    },
    status: {
        type: String,
        required: true,
    },
    progress: {
        type: String,
        default: '',
    },
    error: {
        type: String,
        default: '',
    },
    available: {
        type: Object,
        default: null,
    },
});

defineEmits(['activate']);

const busy = computed(() => props.checking || props.installing);

const actionLabel = computed(() => {
    if (props.installing) {
        return 'Installing';
    }

    if (props.checking) {
        return 'Checking';
    }

    return props.available ? 'Install' : 'Update';
});

const actionDescription = computed(() => {
    if (props.installing) {
        return 'Installing a signed Dust Wave Social update';
    }

    if (props.checking) {
        return 'Checking GitHub Releases for a signed Dust Wave Social update';
    }

    if (props.available) {
        return `Install Dust Wave Social ${props.available.version}`;
    }

    if (props.error) {
        return 'Check for Dust Wave Social updates again';
    }

    return 'Check for Dust Wave Social updates';
});

const detail = computed(() => {
    if (props.progress) {
        return props.progress;
    }

    if (props.status && props.status !== 'Not checked yet') {
        return props.status;
    }

    return '';
});

const state = computed(() => {
    if (props.installing) {
        return 'installing';
    }

    if (props.checking) {
        return 'checking';
    }

    if (props.available) {
        return 'available';
    }

    if (props.error) {
        return 'error';
    }

    return 'idle';
});
</script>

<template>
    <div class="topbar-update-control" :data-state="state">
        <button
            type="button"
            :class="[
                'topbar-update-button',
                {
                    'is-active': available,
                    'is-error': error,
                },
            ]"
            :disabled="busy"
            :aria-busy="busy"
            :aria-label="actionDescription"
            :title="actionDescription"
            @click="$emit('activate')"
        >
            <svg
                class="topbar-update-icon"
                data-icon="arrow-down-circle"
                viewBox="0 0 24 24"
                aria-hidden="true"
            >
                <circle cx="12" cy="12" r="9.25" />
                <path d="M12 7.25v8.25m-3.5-3.25L12 15.75l3.5-3.5" />
            </svg>
            <span>{{ actionLabel }}</span>
        </button>
        <span
            v-if="detail"
            :class="['topbar-update-status', { 'is-error': error }]"
            :role="error ? 'alert' : 'status'"
        >
            {{ detail }}
        </span>
    </div>
</template>
