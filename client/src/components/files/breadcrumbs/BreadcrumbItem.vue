<script setup lang="ts">
interface BreadcrumbItemProps {
    id: number;
    icon?: string;
    title: string;
}

interface BreadcrumbItemEvents {
    (event: "breadcrumb-clicked", id: number): void;
}

const props = defineProps<BreadcrumbItemProps>();
const emit = defineEmits<BreadcrumbItemEvents>();

function onBreadcrumbClick(): void {
    emit("breadcrumb-clicked", props.id);
}
</script>
<template>
    <div class="breadcrumb-item" v-on:click="onBreadcrumbClick">
        <span v-if="!!props.icon" class="icon-filled">{{ props.icon }}</span>
        <span>{{ props.title }}</span>
    </div>
</template>
<style scoped lang="scss">
@use "@/assets/icons.scss";

.breadcrumb-item {
    display: flex;
    align-items: center;
    font-size: 1.4rem;
    column-gap: 4px;

    .icon {
        font-size: 30px;
    }
}

// The last breadcrumb is the selected one.
.breadcrumb-item:not(:last-child) {
    color: var(--breadcrumb-notselected-color);
    cursor: pointer;
    &::after {
        content: "navigate_next";
        @extend .icon-filled;
    }
}
</style>
