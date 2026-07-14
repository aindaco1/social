<script setup>
defineProps({
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
    badge: {
        type: String,
        required: true,
    },
});

defineEmits(['check', 'install']);
</script>

<template>
    <section class="panel">
        <div class="panel-heading">
            <div>
                <h2>Software Updates</h2>
                <p>Signed desktop releases are checked through GitHub Releases.</p>
            </div>
            <div class="health-heading-actions">
                <button
                    type="button"
                    class="inline-button"
                    :disabled="checking || installing"
                    aria-label="Check GitHub Releases for Dust Wave Social updates"
                    @click="$emit('check')"
                >
                    Check
                </button>
                <button
                    type="button"
                    class="inline-button"
                    :disabled="checking || installing || !available"
                    :aria-label="available ? `Install Dust Wave Social ${available.version}` : 'Install Dust Wave Social update'"
                    @click="$emit('install')"
                >
                    Install
                </button>
            </div>
        </div>
        <div v-if="progress" class="form-note" aria-live="polite">{{ progress }}</div>
        <div v-if="error" class="form-error" aria-live="assertive">{{ error }}</div>
        <div class="system-detail-grid">
            <div class="system-detail-row">
                <span>Release channel</span>
                <strong>GitHub Releases</strong>
            </div>
            <div class="system-detail-row">
                <span>Status</span>
                <strong aria-live="polite">{{ status }}</strong>
            </div>
            <div class="system-detail-row">
                <span>Available update</span>
                <strong>{{ available ? available.version : 'None' }}</strong>
            </div>
            <div class="system-detail-row">
                <span>Updater</span>
                <strong>{{ badge }}</strong>
            </div>
        </div>
    </section>
</template>
