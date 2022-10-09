<script setup lang="ts">
import { inject, ref } from "vue";

import { clipboardServiceInjectionToken } from "@/services/files/clipboard-service";

import NodeDropDown from "../dropdown/NodeDropDown.vue";

import type { NodeListController } from "./node-list-controller";
import NodeListHeader from "./NodeListHeader.vue";
import NodeListRow from "./NodeListRow.vue";
import type { FileNode } from "@/models/file-node";

interface NodeListProps {
    controller: NodeListController;
}

defineProps<NodeListProps>();
const clipboardService = inject(clipboardServiceInjectionToken)!;

const nodeMenuRef = ref<FileNode | null>(null);
const nodeMenuTarget = ref<DOMRect>({} as DOMRect);
function onNodeMenuClick(node: FileNode, targetPosition: DOMRect): void {
    if (nodeMenuRef.value && node.id === nodeMenuRef.value.id) {
        // when same click
        nodeMenuRef.value = null;
    } else {
        // different or not set.
        nodeMenuRef.value = node;
        nodeMenuTarget.value = targetPosition;
    }
}
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
            :class="{ 'node-row--clipboard': !!clipboardService.itemsIndex[node.id] }"
            @node-selection-toggled="(node, selected) => controller.toggleNodeSelection(node, selected)"
            @node-favorite-toggled="(node, favorite) => controller.toggleNodeFavorite(node, favorite)"
            @node-title-click="node => controller.nodeTitleClicked(node)"
            @node-menu-click="onNodeMenuClick"
            @node-rename-cancel="node => controller.toggleNodeRename(node, false)"
            @node-rename="(node, name) => controller.renameNode(node, name)"
        />
        <node-drop-down
            v-if="!!nodeMenuRef"
            :node="nodeMenuRef"
            :target-position="nodeMenuTarget"
            :controller="controller"
            @dropdown-close="nodeMenuRef = null"
        />
    </div>
</template>

<style scoped lang="scss">
.node-list {
    display: grid;
    grid-template-columns: 100px 650px 150px 300px 80px;
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
