<script setup lang="ts">
import FilesActionCreate from "./create/FilesActionCreate.vue";

export interface FileActionsProps {
    parentId: number;
    selectedNodes: number;
}

const props = defineProps<FileActionsProps>();
</script>
<template>
    <ul class="file-actions">
        <template v-if="props.selectedNodes === 0">
            <li class="file-actions-create-container">
                <files-action-create />
            </li>
        </template>
        <template v-if="props.selectedNodes > 0">
            <li>
                <button class="icon-button ghost-button">
                    <span class="icon-outlined">file_download</span>
                    Download
                </button>
            </li>
            <li>
                <button class="icon-button ghost-button">
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
        <template v-if="props.selectedNodes === 1">
            <li>
                <button class="icon-button ghost-button">
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
    align-items: center;
    padding: 0 10px;
    border-bottom: 1px solid var(--border-color);

    > li {
        flex: 1;
        max-width: auto;
    }
}

.file-actions-create-container {
    position: relative;
}
</style>
