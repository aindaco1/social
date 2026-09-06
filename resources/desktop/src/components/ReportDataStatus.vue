<script setup>
import { formatDateOnly } from '../dateTime.js';
defineProps({ report: Object, settings: Object, busy: Boolean, error: String, canImport: Boolean });
defineEmits(['import']);
</script>

<template>
    <div class="report-data-status" role="status">
        <div>
            <p v-if="report?.latest_observation_date">Latest available observation: {{ formatDateOnly(report.latest_observation_date, settings?.date_format) }}.</p>
            <p v-else>No observations imported for this account yet.</p>
            <small>Missing measurements are shown as “No data”, not zero. Totals use available imported observations, which may be incomplete. Daily totals keep the provider’s reporting dates.</small>
            <p v-if="error" class="form-error">{{ error }}</p>
        </div>
        <button v-if="canImport" type="button" class="inline-button" :disabled="busy" @click="$emit('import')">{{ busy ? 'Importing…' : 'Import latest data' }}</button>
    </div>
</template>
