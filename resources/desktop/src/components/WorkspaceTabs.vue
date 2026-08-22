<script setup>
defineProps({
    modelValue: {
        type: String,
        required: true,
    },
    tabs: {
        type: Array,
        required: true,
    },
    label: {
        type: String,
        required: true,
    },
});

const emit = defineEmits(['update:modelValue']);
</script>

<template>
    <div class="workspace-mode-tabs" role="tablist" :aria-label="label">
        <button
            v-for="tab in tabs"
            :key="tab.id"
            type="button"
            role="tab"
            :aria-selected="modelValue === tab.id"
            :class="{ 'is-active': modelValue === tab.id }"
            @click="emit('update:modelValue', tab.id)"
        >
            {{ tab.label }}
        </button>
    </div>
</template>
