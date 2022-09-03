<script setup lang="ts">
import { computed } from "vue";

import { type FileNode, NodeType } from "@/models/file-node";

import SelectAction from "./actions/SelectAction.vue";
import FavoriteAction from "./actions/FavoriteAction.vue";
import type { NodeState } from "./node-list-controller";

interface NodeListRowProps {
    node: FileNode;
    state: NodeState;
}

interface NodeListRowEvent {
    (event: "node-selection-toggled", node: FileNode, selected: boolean): void;
    (event: "node-favorite-toggled", node: FileNode, favorite: boolean): void;
    (event: "node-title-click", node: FileNode): void;
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
</script>

<template>
    <div class="node-row">
        <div class="node-row__actions">
            <select-action :is-selected="state.selected" @selection-toggled="onNodeSelectionToggled" />
            <favorite-action :is-favorite="node.isFavorite" @favorite-toggled="onNodeFavoriteToggled" />
        </div>
        <div class="node-row__title">
            <span class="icon-filled">{{ nodeIcon }}</span>
            <input class="input node-row__title__name-input" type="text" v-if="state.rename" :value="node.title" />
            <span class="node-row__title__name" @click="onNodeTitleClick" v-else>{{ node.title }}</span>
            <span class="icon-filled file-item-menu node-row-action">more_vert</span>
        </div>
        <div class="node-row__node-size">{{ nodeSize }}</div>
        <div class="node-row__modified_at">{{ node.modifiedAt.toLocaleString() }}</div>
    </div>
</template>

<style scoped lang="scss">
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
}
</style>
