<script setup lang="ts">
import { inject } from "vue";

import { fileActionServiceInjectionToken } from "@/services/files/file-action-service";
import { fileSystemServiceInjectionToken } from "@/services/files/file-system-service";
import { ClipboardOperation } from "@/services/files/clipboard-service";

import type { NodeListController } from "../list/node-list-controller";
import FilesActionCreate from "./create/FilesActionCreate.vue";
import CopyAction from "./copy/CopyAction.vue";

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

async function onUploadFolder() {
    const folder = await fs.loadFolder();
    await fileActionService.uploadFolder(props.parentId, folder);
    await props.ctrl.refresh();
}

async function onDeleteSelectedNodes() {
    for (const node of props.ctrl.selectedNodes.value) {
        await fileActionService.deleteNode(node.id);
    }
    await props.ctrl.refresh();
}

async function onDownloadSelectedNodes() {
    await fileActionService.downloadNodes(props.ctrl.selectedNodes.value);
}
</script>
<template>
    <ul class="file-actions">
        <li class="file-actions-item file-actions-create-container">
            <files-action-create
                class="file-actions-button"
                @create-folder="onCreateFolder"
                @upload-files="onUploadFiles"
                @upload-folder="onUploadFolder"
            />
        </li>
        <template v-if="ctrl.selectedNodes.value.length > 0">
            <li class="file-actions-item">
                <button class="file-actions-button icon-button ghost-button" @click="onDownloadSelectedNodes">
                    <span class="icon-outlined">file_download</span>
                    Download
                </button>
            </li>
            <li class="file-actions-item">
                <button class="file-actions-button icon-button ghost-button" @click="onDeleteSelectedNodes">
                    <span class="icon-outlined">delete</span>
                    Delete
                </button>
            </li>
            <li class="file-actions-item">
                <copy-action
                    class="file-actions-button"
                    :ctrl="ctrl"
                    :operation="ClipboardOperation.Cut"
                    :selected-nodes="ctrl.selectedNodes"
                />
            </li>
            <li class="file-actions-item">
                <copy-action
                    class="file-actions-button"
                    :ctrl="ctrl"
                    :operation="ClipboardOperation.Copy"
                    :selected-nodes="ctrl.selectedNodes"
                />
            </li>
        </template>
    </ul>
</template>

<style>
.file-actions {
    flex: 0 0 50px;
    display: flex;
    flex-flow: row nowrap;
    align-items: center;
    padding: 0 10px;
    border-bottom: 1px solid var(--border-color);
}

.file-actions-item {
    width: 120px;
}

.file-actions-button {
    margin: 0 auto;
    font-size: 15px;
}

.file-actions-create-container {
    position: relative;
}

@media (max-width: 900px) {
    .file-actions {
        /* overflow-x: auto; */
    }

    .file-actions-button {
        padding: 0 !important;
        margin: 0 10px 0 0;
    }
}
</style>
