<script setup lang="ts">
import NodeListHeader from "./NodeListHeader.vue";
import NodeListRow from "./NodeListRow.vue";

import type { NodeListController } from "@/components/files/list/node-list-controller";

export interface NodeListProps {
    controller: NodeListController;
}

const props = defineProps<NodeListProps>();
</script>

<template>
    <div class="node-list">
        <node-list-header
            :is-all-rows-selected="props.controller.allNodesSelected.value"
            @select-all-toggled="selected => props.controller.toggleAllNodeSelection(selected)"
        />
        <node-list-row
            v-for="node in props.controller.nodes"
            :key="node.id"
            :node="node"
            :state="props.controller.nodesState[node.id]"
            @node-selection-toggled="(node, selected) => props.controller.toggleNodeSelection(node, selected)"
            @node-title-click="node => props.controller.nodeTitleClicked(node)"
        />
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
