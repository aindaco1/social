<script setup>
import { nextTick, ref } from 'vue';
import { localMediaStatus } from '@desktop/localMediaOperation.js';

defineProps({
    result: { type: Object, default: null },
    error: { type: String, default: '' },
    canCancel: { type: Boolean, default: false },
    draft: { type: Object, default: null },
});
const emit = defineEmits(['cancel', 'update:draft']);
const feedback = ref(null);
const dismissControl = async (event, value) => {
    emit(event, value);
    await nextTick();
    feedback.value?.focus({ preventScroll: true });
};
</script>

<template>
    <div ref="feedback" class="local-ai-feedback" tabindex="-1" aria-label="Local media feedback">
        <p class="form-note" role="status" aria-live="polite" aria-atomic="true">{{ result?.result?.alt_text && !draft ? 'Alt-text draft discarded.' : localMediaStatus(result) }}</p>
        <div v-if="canCancel" class="local-ai-progress">
            <button type="button" class="inline-button" @click="dismissControl('cancel')">Cancel</button>
        </div>
        <p role="alert" aria-atomic="true" :class="{ 'form-error': error }">{{ error }}</p>
        <details v-if="result?.status === 'complete' && result.result?.timings" class="form-note">
            <summary>Processing details</summary>
            <p>Model loading: {{ (result.result.timings.compile_ms / 1000).toFixed(1) }}s</p>
            <template v-if="result.result.timings.execution">
                <p v-if="result.result.timings.execution.backend === 'native_litert'">Native LiteRT {{ result.result.timings.execution.runtime_version }} · {{ result.result.timings.execution.cpu_threads }} CPU threads</p>
                <p v-else>CPU threads: {{ result.result.timings.execution.cpu_threads }} · shared memory: {{ result.result.timings.execution.shared_memory ? 'available' : 'unavailable' }} · cross-origin isolation: {{ result.result.timings.execution.cross_origin_isolated ? 'on' : 'off' }}</p>
            </template>
            <template v-if="result.result.timings.tiles">
                <p>Inference: {{ (result.result.timings.inference_ms / 1000).toFixed(1) }}s · {{ result.result.timings.tiles }} tile(s)</p>
                <p>Processing total: {{ (result.result.timings.elapsed_ms / 1000).toFixed(1) }}s · longest UI timer gap: {{ result.result.timings.ui_max_gap_ms }}ms (100ms sampling)</p>
                <p>Session-only diagnostics, excluding derivative saving. Background windows may throttle timers.</p>
            </template>
        </details>
        <div v-if="draft" class="local-ai-alt-text">
            <label for="local-ai-alt-text">Generated alt-text draft · {{ draft.name }}</label>
            <textarea id="local-ai-alt-text" :value="draft.text" rows="3" aria-describedby="local-ai-alt-text-review" @input="$emit('update:draft', { ...draft, text: $event.target.value })" />
            <p id="local-ai-alt-text-review" class="form-note">Based on image properties, not a description of its contents. Review and edit before copying. This temporary draft is not saved or attached to posts.</p>
            <div><button type="button" class="inline-button" @click="dismissControl('update:draft', null)">Discard draft</button></div>
        </div>
    </div>
</template>
