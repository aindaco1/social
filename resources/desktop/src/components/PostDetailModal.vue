<script setup>
import ProviderPreviewCard from './ProviderPreviewCard.vue';

defineProps({
    summary: {
        type: Object,
        required: true,
    },
    detail: {
        type: Object,
        default: null,
    },
    loading: {
        type: Boolean,
        default: false,
    },
    error: {
        type: String,
        default: '',
    },
    accountsCount: {
        type: Number,
        default: 0,
    },
    customVersionCount: {
        type: Number,
        default: 0,
    },
    tags: {
        type: Array,
        default: () => [],
    },
    mediaCount: {
        type: Number,
        default: 0,
    },
    mediaLabel: {
        type: String,
        default: 'None',
    },
    previewCards: {
        type: Array,
        default: () => [],
    },
    timeline: {
        type: Array,
        default: () => [],
    },
});

const emit = defineEmits(['close']);
</script>

<template>
    <div
        class="modal-backdrop"
        role="dialog"
        aria-modal="true"
        aria-labelledby="post-detail-title"
        @click.self="emit('close')"
    >
        <div class="post-detail-modal">
            <header>
                <div>
                    <h3 id="post-detail-title">Post details</h3>
                    <small>{{ summary.uuid }}</small>
                </div>
                <button type="button" class="modal-close-button" aria-label="Close post details" @click="emit('close')">
                    &times;
                </button>
            </header>
            <div v-if="loading" class="form-note">Loading post detail</div>
            <div v-if="error" class="form-error">{{ error }}</div>
            <template v-if="detail">
                <div class="post-detail-summary">
                    <div>
                        <span>Status</span>
                        <strong>{{ detail.status }}</strong>
                        <small>{{ detail.schedule_status }}</small>
                    </div>
                    <div>
                        <span>Accounts</span>
                        <strong>{{ accountsCount }}</strong>
                        <small>{{ customVersionCount }} custom version(s)</small>
                    </div>
                    <div>
                        <span>Labels</span>
                        <strong>{{ tags.length }}</strong>
                        <small>{{ tags.map((tag) => tag.name).join(', ') || 'None' }}</small>
                    </div>
                    <div>
                        <span>Media</span>
                        <strong>{{ mediaCount }}</strong>
                        <small>{{ mediaLabel }}</small>
                    </div>
                </div>
                <div class="post-detail-body">
                    <section>
                        <h4>Provider Previews</h4>
                        <div class="provider-preview-grid">
                            <ProviderPreviewCard
                                v-for="preview in previewCards"
                                :key="preview.account.uuid"
                                :preview="preview"
                            />
                        </div>
                    </section>
                    <section>
                        <h4>History</h4>
                        <div v-if="timeline.length" class="post-history-list">
                            <article
                                v-for="item in timeline"
                                :key="`${item.label}-${item.at}-${item.detail}`"
                                :class="['post-history-item', `is-${item.tone}`]"
                            >
                                <span></span>
                                <div>
                                    <strong>{{ item.label }}</strong>
                                    <small>{{ item.at }}</small>
                                    <p>{{ item.detail }}</p>
                                </div>
                            </article>
                        </div>
                        <div v-else class="empty-inline">No history available</div>
                        <div v-if="tags.length" class="composer-tag-list">
                            <span
                                v-for="tag in tags"
                                :key="tag.uuid"
                                class="composer-tag-chip"
                            >
                                <span class="tag-swatch" :style="{ backgroundColor: `#${tag.hex_color}` }"></span>
                                {{ tag.name }}
                            </span>
                        </div>
                    </section>
                </div>
            </template>
        </div>
    </div>
</template>
