<script setup lang="ts">
import { ref } from "vue";
import NewFolderAction from "./NewFolderAction.vue";
import UploadFileAction from "./UploadFileAction.vue";

const listVisible = ref(false);

interface FileActionCreateEvents {
    (event: "create-folder", name: string): void;
    (event: "upload-files"): void;
}

const emit = defineEmits<FileActionCreateEvents>();

function createFolder(name: string) {
    emit("create-folder", name);
    listVisible.value = false;
}

function uploadFiles() {
    emit("upload-files");
    listVisible.value = false;
}
</script>
<template>
    <button class="file-action-create ghost-button" v-on:click="listVisible = !listVisible" :class="$attrs.class">
        <span class="icon-outlined">note_add</span>
        <span>Create</span>
        <span class="icon-filled">arrow_drop_down</span>
    </button>
    <!-- eslint-disable max-len -->
    <ul class="file-action-create-list popup" v-if="listVisible">
        <li class="file-action-create-list-item file-action-create-list-item--end-group">
            <new-folder-action @create-folder="createFolder" />
        </li>
        <li class="file-action-create-list-item file-action-create-list-item--start-group">
            <upload-file-action parent_id="{props.parent_id}" @upload-files="uploadFiles" />
        </li>
        <!-- <li class="file-action-create-list-item file-action-create-list-item--start-group">
            <a>
                <span class="icon-outlined">description</span>
                <span>Text file</span>
            </a>
        </li> -->
    </ul>
</template>
<style lang="scss">
.file-action-create {
    padding: 2px 7px;
    display: flex;
    column-gap: 5px;
    align-items: center;
    font-size: 15px;
}

.file-action-create-list {
    position: absolute;
    left: 7px;
    width: 250px;
    padding: 10px 0;
}

.file-action-create-list-item {
    > a {
        display: flex;
        align-items: center;
        column-gap: 7px;

        height: 35px;
        padding: 0 15px;
        cursor: pointer;

        &:hover {
            background: var(--list-item-hover-color);
        }
    }
}

.file-action-create-list-item--start-group {
    &::before {
        content: "";
        display: block;
        height: 5px;
    }
}

.file-action-create-list-item--end-group {
    &::after {
        content: "";
        display: block;
        height: 5px;
        border-bottom: 1px solid var(--border-color);
    }
}

@media (max-width: 900px) {
    .file-action-create {
        padding: 0;
    }
}
</style>
