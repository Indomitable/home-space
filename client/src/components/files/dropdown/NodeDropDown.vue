<script setup lang="ts">
import { computed, inject, type StyleValue, watchEffect } from "vue";
import type { FileNode } from "@/models/file-node";
import { ClipboardOperation, clipboardServiceInjectionToken } from "@/services/files/clipboard-service";
import type { NodeListController } from "@/components/files/list/node-list-controller";

export interface NodeDropDownProps {
    node: FileNode;
    targetPosition: DOMRect;
    controller: NodeListController;
}

export interface NodeDropDownEvents {
    (event: "dropdown-close"): void;
}

const props = defineProps<NodeDropDownProps>();

const pos = computed<StyleValue>(() => {
    const popupSize = { width: 130, height: 130 };
    const position: StyleValue = {};
    const height = window.innerHeight;
    const width = window.innerWidth;
    if (props.targetPosition.left + popupSize.width > width) {
        // The popup goes outside of screen - show it to the left.
        // target right = distance from left, but our right is distance from right.
        position.right = `${width - props.targetPosition.right}px`;
    } else {
        position.left = `${props.targetPosition.left}px`;
    }

    if (props.targetPosition.top + popupSize.height > height) {
        // The popup goes outside of screen - show it to the top.
        position.bottom = `${height - props.targetPosition.top + 5}px`;
    } else {
        position.top = `${props.targetPosition.bottom + 5}px`;
    }
    return position;
});

const emits = defineEmits<NodeDropDownEvents>();
function onBodyClick(event: MouseEvent) {
    if (!event.target || !(event.target as HTMLElement).classList.contains("file-item-menu")) {
        // we don't want to emit dropdown close when clicking on the file-item-menu.
        emits("dropdown-close");
    }
}

watchEffect(onCleanup => {
    document.addEventListener("click", onBodyClick);
    onCleanup(() => {
        document.removeEventListener("click", onBodyClick);
    });
});

function onRename() {
    props.controller.toggleNodeRename(props.node, true);
}

const clipboardService = inject(clipboardServiceInjectionToken)!;
const parentId = inject("parent-id") as number;

function onCutClick() {
    clipboardService.addToClipboard(parentId, [props.node], ClipboardOperation.Cut);
}

function onCopyClick() {
    clipboardService.addToClipboard(parentId, [props.node], ClipboardOperation.Copy);
}
</script>

<template>
    <ul class="node-dropdown-menu" :style="pos">
        <li class="node-dropdown-menu-item" @click="onRename">Rename</li>
        <li class="node-dropdown-menu-item" @click="onCutClick" v-if="!clipboardService.hasItems.value">Cut</li>
        <li class="node-dropdown-menu-item" @click="onCopyClick" v-if="!clipboardService.hasItems.value">Copy</li>
    </ul>
</template>

<style>
.node-dropdown-menu {
    position: absolute;
    background: var(--background-color);
    z-index: var(--z-index-node-dropdown);
    padding: 2px 0;
    border: 1px solid rgb(66, 66, 66);
    box-shadow: 3px 4px 5px 0 rgb(171 171 171 / 20%);
}
.node-dropdown-menu-item {
    height: 40px;
    line-height: 40px;
    width: 120px;
    padding: 0 20px;
    cursor: pointer;
}
.node-dropdown-menu-item:hover {
    background: var(--list-item-hover-color);
}
</style>
