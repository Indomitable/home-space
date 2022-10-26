<script setup lang="ts">
import { inject } from "vue";
import { ClipboardOperation, clipboardServiceInjectionToken } from "@/services/files/clipboard-service";
import { type FileNode, NodeType } from "@/models/file-node";

const clipboardService = inject(clipboardServiceInjectionToken)!;
const parentId = inject("parent-id") as number;

function onPaste() {
    clipboardService.paste(parentId);
}

function onCancel() {
    clipboardService.clear();
}
function nodeIcon(node: FileNode): string {
    return node.nodeType === NodeType.Folder ? "folder" : "insert_drive_file";
}
</script>

<template>
    <Teleport to="body" v-if="clipboardService.hasItems.value">
        <div class="clipboard-container">
            <div class="clipboard-container-title">
                {{
                    `${clipboardService.items.length} file(s) ${
                        clipboardService.operation === ClipboardOperation.Cut ? "moved" : "copied"
                    } to clipboard:`
                }}
            </div>
            <ul class="clipboard-container-items">
                <li v-for="node of clipboardService.items" :key="node.id" class="clipboard-container-item" :title="node.title">
                    <span class="clipboard-container-item__icon icon-filled">{{ nodeIcon(node) }}</span>
                    <span class="clipboard-container-item__title">{{ node.title }}</span>
                </li>
            </ul>
            <div class="clipboard-container-actions">
                <button class="button" @click="onPaste" :disabled="clipboardService.parentId === parentId">Paste</button>
                <button class="button" @click="onCancel">Cancel</button>
            </div>
        </div>
    </Teleport>
</template>

<style>
.clipboard-container {
    position: fixed;
    bottom: 0;
    right: 10px;
    background: var(--invert-background-color);
    color: var(--invert-font-color);
    padding: 10px;
    width: 500px;
    height: 300px;
    display: flex;
    flex-direction: column;
    z-index: var(--z-index-clipboard);
}

.clipboard-container-title {
    padding-bottom: 15px;
}

.clipboard-container-items {
    flex: 1;
}

.clipboard-container-item {
    height: 40px;
    display: flex;
    flex-direction: row;
    column-gap: 3px;
    align-items: center;
    white-space: nowrap;
}

.clipboard-container-item__icon {
    color: var(--invert-font-color);
}

.clipboard-container-item__title {
    text-overflow: ellipsis;
    overflow: hidden;
    flex: 1;
}

.clipboard-container-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
}

@media (max-width: 900px) {    
    .clipboard-container {
        right: 0;
        width: 400px;
    }
}

@media (max-width: 400px) {
    .clipboard-container {
        width: 100%;
        height: 250px;
    }
}

@media (max-height: 700px) {
    .clipboard-container-title {
        padding-bottom: 5px;
    }
    .clipboard-container {
        height: 180px;
    }
}
</style>
