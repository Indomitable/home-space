<script setup lang="ts">
import { inject } from "vue";

import { fileActionServiceInjectionToken } from "@/services/files/file-action-service";
import { fileSystemServiceInjectionToken } from "@/services/files/file-system-service";

import type { NodeListController } from "../list/node-list-controller";
import FilesActionCreate from "./create/FilesActionCreate.vue";

export interface FileActionsProps {
    parentId: number;
    ctrl: NodeListController;
}

const props = defineProps<FileActionsProps>();

const fileActionService = inject(fileActionServiceInjectionToken)!;
async function onCreateFolder(name: string) {
    await fileActionService.createFolder(props.parentId, name);
    await props.ctrl.refresh();
}

const fs = inject(fileSystemServiceInjectionToken)!;
async function onUploadFiles() {
    const files = fs.loadFiles([]);
    for await (const file of files) {
        await fileActionService.uploadFile(props.parentId, file);
    }
    await props.ctrl.refresh();
}

async function onDeleteSelectedNodes() {
    for (const node of props.ctrl.selectedNodes.value) {
        await fileActionService.deleteNode(node.id);
    }
}
</script>
<template>
    <ul class="file-actions">
        <li class="file-actions-create-container">
            <files-action-create @create-folder="onCreateFolder" @upload-files="onUploadFiles" />
        </li>
        <template v-if="ctrl.selectedNodes.value.length > 0">
            <li>
                <button class="icon-button ghost-button">
                    <span class="icon-outlined">file_download</span>
                    Download
                </button>
            </li>
            <li>
                <button class="icon-button ghost-button" @click="onDeleteSelectedNodes">
                    <span class="icon-outlined">delete</span>
                    Delete
                </button>
            </li>
            <li>
                <button class="icon-button ghost-button">
                    <span class="icon-outlined">drive_file_move</span>
                    Move to
                </button>
            </li>
            <li>
                <button class="icon-button ghost-button">
                    <span class="icon-outlined">file_copy</span>
                    Copy to
                </button>
            </li>
        </template>
        <template v-if="ctrl.selectedNodes.value.length === 1">
            <li>
                <button
                    class="icon-button ghost-button"
                    @click="() => ctrl.toggleNodeRename(ctrl.selectedNodes.value[0], true)"
                >
                    <span class="icon-outlined">drive_file_rename_outline</span>
                    Rename
                </button>
            </li>
        </template>
    </ul>
</template>

<style scoped lang="scss">
.file-actions {
    height: 50px;
    display: flex;
    flex-flow: row nowrap;
    align-items: center;
    padding: 0 10px;
    border-bottom: 1px solid var(--border-color);

    > li {
        width: 120px;

        > button {
            margin: 0 auto;
            font-size: 15px;
        }
    }
}

.file-actions-create-container {
    position: relative;
}
</style>
