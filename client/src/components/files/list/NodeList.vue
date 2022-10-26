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

const props = defineProps<NodeListProps>();
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

const dragTarget = ref<HTMLDivElement | null>(null);

function onNodeListDragLeave() {
    if (dragTarget.value) {
        dragTarget.value.classList.toggle("node-list--drag-target", false);
    }
}

function onNodeListDrop(event: DragEvent) {
    event.preventDefault();
    if (event.dataTransfer && event.dataTransfer.files.length > 0) {
        const files = event.dataTransfer.files;
        for (let i = 0; i < files.length; i++) {
            const file = files.item(i);
            if (file !== null) {
                props.controller.renameNode;
            }
        }
    }
}

function onNodeListDragOver(event: DragEvent) {
    if (dragTarget.value && event.dataTransfer && event.dataTransfer.items.length > 0) {
        const items = event.dataTransfer.items;
        for (let i = 0; i < items.length; i++) {
            const item = items[i];
            if (item.kind === "file") {
                dragTarget.value.classList.toggle("node-list--drag-target", true);
                event.preventDefault();
                break;
            }
        }
    }
}
</script>

<template>
    <div ref="dragTarget" class="node-list" @dragleave="onNodeListDragLeave" @drop="onNodeListDrop" @dragover="onNodeListDragOver">
        <node-list-header
            :is-all-rows-selected="controller.allNodesSelected.value"
            :sorting="controller.sorting.value"
            @select-all-toggled="selected => controller.toggleAllNodeSelection(selected)"
            @sort-changed="sorting => controller.load(sorting)"
        />
        <div class="node-list-rows">
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
    </div>
</template>

<style>
.node-list {
    max-width: 1300px;
    display: flex;
    flex-direction: column;
    height: 100%;
}
.node-list-header,
.node-list-rows {
    display: grid;
    grid-template-columns: 10% 40% 20% 20% 10%;
    width: 100%;
}

.node-list-rows {
    overflow-y: auto;
}

.node-list--drag-target {
    background-color: #4a3f54;
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
    .node-list-header,
    .node-list-rows {
        grid-template-columns: minmax(50%, 70%) 1fr;
    }
}

@media (max-width: 400px) {
    .node-list-header,
    .node-list-rows {
        grid-template-columns: 100%;
    }
}
</style>
