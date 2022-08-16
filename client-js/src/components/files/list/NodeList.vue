<script setup lang="ts">
import type { FileNode } from "@/services/files/files-load-service";
import NodeListHeader from "./NodeListHeader.vue";
import NodeListRow from "./NodeListRow.vue";

export interface NodeListProps {
    nodes: FileNode[];
}

const props = defineProps<NodeListProps>();
</script>

<template>
    <div class="node-list">
        <node-list-header :is-all-rows-selected="false" />
        <node-list-row v-for="node in props.nodes" :key="node.id" :node="node" />
    </div>
</template>

<style scoped lang="scss">
.node-list {
    display: grid;
    grid-template-columns: 100px auto 150px 300px;
}

:deep(.node-row-action) {
    visibility: hidden;
    user-select: none;
}

:deep(.node-row-action--visible) {
    visibility: visible;
}
</style>
