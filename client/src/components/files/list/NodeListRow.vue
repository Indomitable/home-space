<script setup lang="ts">
import { computed, ref, watchEffect } from "vue";

import { type FavoriteFileNode, type FileNode, NodeType } from "@/models/file-node";

import SelectAction from "./actions/SelectAction.vue";
import FavoriteAction from "./actions/FavoriteAction.vue";
import type { NodeState } from "./node-list-controller";

export interface NodeListRowProps {
    node: FavoriteFileNode;
    state: NodeState;
}

export interface NodeListRowEvent {
    (event: "node-selection-toggled", node: FileNode, selected: boolean): void;
    (event: "node-favorite-toggled", node: FileNode, favorite: boolean): void;
    (event: "node-title-click", node: FileNode): void;
    (event: "node-menu-click", node: FileNode, targetPosition: DOMRect): void;
    (event: "node-rename-cancel", node: FileNode): void;
    (event: "node-rename", node: FileNode, newName: string): void;
}

const props = defineProps<NodeListRowProps>();
const emits = defineEmits<NodeListRowEvent>();

const nodeIcon = computed(() => (props.node.nodeType === NodeType.Folder ? "folder" : "insert_drive_file"));
const nodeSize = computed(() => (props.node.nodeType === NodeType.File ? props.node.nodeSizeHuman : ""));

function onNodeSelectionToggled(selected: boolean) {
    emits("node-selection-toggled", props.node, selected);
}

function onNodeFavoriteToggled(favorite: boolean) {
    emits("node-favorite-toggled", props.node, favorite);
}

function onNodeTitleClick() {
    emits("node-title-click", props.node);
}
function onNodeMenuClick(event: MouseEvent) {
    const target = event.target! as HTMLElement;
    const targetPosition = target.getBoundingClientRect();
    emits("node-menu-click", props.node, targetPosition);
}

const renameInput = ref<HTMLInputElement | null>(null);
watchEffect(
    () => {
        if (renameInput.value) {
            renameInput.value.focus();
        }
    },
    { flush: "post" }
);
function onNodeRename(event: KeyboardEvent) {
    const input = event.target as HTMLInputElement;
    if (event.key === "Enter") {
        emits("node-rename", props.node, input.value);
    }
    if (event.key === "Escape") {
        emits("node-rename-cancel", props.node);
    }
}
</script>

<template>
    <div class="node-row">
        <div class="node-row__actions">
            <select-action :is-selected="state.selected" @selection-toggled="onNodeSelectionToggled" />
            <favorite-action :is-favorite="node.isFavorite" @favorite-toggled="onNodeFavoriteToggled" />
        </div>
        <div class="node-row__title">
            <span class="icon-filled">{{ nodeIcon }}</span>
            <input
                class="input node-row__title__name-input"
                type="text"
                v-if="state.rename"
                :value="node.title"
                @blur="$emit('node-rename-cancel', node)"
                @keyup="onNodeRename"
                ref="renameInput"
            />
            <span class="node-row__title__name" @click="onNodeTitleClick" v-else>{{ node.title }}</span>
            <span class="icon-filled file-item-menu node-row-action" @click="onNodeMenuClick">more_vert</span>
        </div>
        <div class="node-row__node-size">{{ nodeSize }}</div>
        <div class="node-row__modified_at">{{ node.modifiedAtDisplay }}</div>
        <div class="node-row__version">{{ node.nodeVersion }}</div>
    </div>
</template>

<style lang="scss">
.node-row {
    display: contents;
    cursor: pointer;

    > div {
        height: 40px;
        line-height: 40px;
    }

    &:hover {
        > div {
            background: var(--list-item-hover-color);
        }

        .node-row-action {
            visibility: visible;
        }
    }
}

.node-row--clipboard {
    > .node-row__title,
    > .node-row__node-size,
    > .node-row__modified_at {
        opacity: 0.4;
    }
}

.node-row__actions {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: center;
    column-gap: 10px;
}

.node-row__title {
    display: flex;
    align-items: center;
    column-gap: 5px;
    max-width: 650px;
}

.node-row__title__name {
    cursor: pointer;
    white-space: nowrap;
    text-overflow: ellipsis;
    overflow: hidden;
}

.node-row__title__name-input {
    height: 30px;
    width: 100%;
}

.node-row__title__name:hover {
    text-decoration: underline;
}

.file-item-menu {
    margin-left: auto;
    margin-right: 15px;
}

.node-row__node-size {
    text-align: right;
}

.node-row__modified_at {
    text-align: center;
    white-space: nowrap;
}
.node-row__version {
    text-align: center;
}

@media (min-width: 901px) and (max-width: 1200px) {
    .node-row {
        font-size: 13px;
    }
}

@media (min-width: 401px) and (max-width: 900px) {
    .node-row {
        font-size: 13px;
    }
    .node-row__actions,
    .node-row__node-size,
    .node-row__version {
        display: none;
    }
}

@media (max-width: 400px) {
    .node-row {
        font-size: 14px;
    }
    .node-row__actions,
    .node-row__node-size,
    .node-row__modified_at,
    .node-row__version {
        display: none;
    }
}
</style>
