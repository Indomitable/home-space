<script setup lang="ts">
import NodeListHeader from "./NodeListHeader.vue";
import NodeListRow from "./NodeListRow.vue";

import type { NodeListController } from "@/components/files/list/node-list-controller";

interface NodeListProps {
    controller: NodeListController;
}

defineProps<NodeListProps>();
</script>

<template>
    <div class="node-list">
        <node-list-header
            :is-all-rows-selected="controller.allNodesSelected.value"
            :sorting="controller.sorting.value"
            @select-all-toggled="selected => controller.toggleAllNodeSelection(selected)"
            @sort-changed="sorting => controller.load(sorting)"
        />
        <node-list-row
            v-for="node in controller.nodes.value"
            :key="node.id"
            :node="node"
            :state="controller.nodesState[node.id]"
            @node-selection-toggled="(node, selected) => controller.toggleNodeSelection(node, selected)"
            @node-favorite-toggled="(node, favorite) => controller.toggleNodeFavorite(node, favorite)"
            @node-title-click="node => controller.nodeTitleClicked(node)"
        />
    </div>
</template>

<style scoped lang="scss">
.node-list {
    display: grid;
    grid-template-columns: 100px 650px 150px 300px;
}

:deep(.node-row-action) {
    visibility: hidden;
    user-select: none;
    cursor: pointer;
}

:deep(.node-row-action--visible) {
    visibility: visible;
}
</style>
