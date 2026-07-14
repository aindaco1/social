<script setup>
import emojiData from 'emoji-mart-vue-fast/data/apple.json';
import 'emoji-mart-vue-fast/css/emoji-mart.css';
import { EmojiIndex, Picker } from 'emoji-mart-vue-fast/src';

const emojiIndex = new EmojiIndex(emojiData);

defineEmits(['select']);
</script>

<template>
    <Picker
        :data="emojiIndex"
        :auto-focus="true"
        :show-preview="false"
        :native="true"
        @select="$emit('select', $event)"
    />
</template>
