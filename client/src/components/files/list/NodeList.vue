<script setup lang="ts">
import { inject, ref } from "vue";

import { clipboardServiceInjectionToken } from "@/services/files/clipboard-service";

import NodeDropDown from "../dropdown/NodeDropDown.vue";

import type { NodeListController } from "./node-list-controller";
import NodeListHeader from "./NodeListHeader.vue";
import NodeListRow from "./NodeListRow.vue";
import type { FileNode } from "@/models/file-node";

export interface NodeListProps {
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
            @node-selection-toggled="(node: FileNode, selected: boolean) => controller.toggleNodeSelection(node, selected)"
            @node-favorite-toggled="(node: FileNode, favorite: boolean) => controller.toggleNodeFavorite(node, favorite)"
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

<style>
.node-list {
    display: grid;
    grid-template-columns: 10% 40% 20% 20% 10%;
    max-width: 1300px;
}

.node-row-action {
    visibility: hidden;
    user-select: none;
    cursor: pointer;
}

.node-row-action--visible {
    visibility: visible;
}

@media (max-width: 900px) {
    .node-list {
        grid-template-columns: minmax(50%, 70%) 1fr;
    }
}

@media (max-width: 400px) {
    .node-list {
        grid-template-columns: 100%;
    }
}
</style>
