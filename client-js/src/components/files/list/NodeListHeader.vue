<script setup lang="ts">
import SelectAction from "./actions/SelectAction.vue";

export interface NodeListHeaderProps {
    isAllRowsSelected: boolean;
}

export interface NodeListHeaderEvents {
    (event: "select-all-toggled", value: boolean): void;
}

const props = defineProps<NodeListHeaderProps>();
const emit = defineEmits<NodeListHeaderEvents>();

function onSelectAllToggled() {
    emit("select-all-toggled", props.isAllRowsSelected);
}
</script>

<template>
    <div class="node-list-header">
        <div class="node-list-header__actions">
            <select-action :is-selected="props.isAllRowsSelected" @selection-toggled="onSelectAllToggled" />
        </div>
        <div class="node-list-header__title">Name</div>
        <div class="node-list-header__title">Size</div>
        <div class="node-list-header__title">Last Modified</div>
    </div>
</template>

<style scoped lang="scss">
.node-list-header {
    display: contents;

    &:hover {
        .node-row-action {
            visibility: visible;
        }
    }
}

.node-list-header__title {
    height: 40px;
    line-height: 40px;
    text-align: center;
    border-bottom: 1px solid var(--border-color);
}

.node-list-header__actions {
    @extend .node-list-header__title;
    padding-left: 22px;
    display: flex;
    align-items: center;
    justify-content: flex-start;
}
</style>
