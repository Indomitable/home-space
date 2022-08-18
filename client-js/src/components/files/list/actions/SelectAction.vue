<script setup lang="ts">
export interface SelectActionProps {
    isSelected: boolean;
}

export interface SelectActionEvents {
    (event: "selection-toggled", selected: boolean): void;
}

const props = defineProps<SelectActionProps>();
const emit = defineEmits<SelectActionEvents>();
function onSelectionClick() {
    emit("selection-toggled", !props.isSelected);
}
</script>

<template>
    <span
        v-if="props.isSelected"
        @click="onSelectionClick"
        class="icon-outlined node-row-action node-row-action--visible"
        >check_box</span
    >
    <span v-else class="icon-outlined node-row-action" @click="onSelectionClick">check_box_outline_blank</span>
</template>
