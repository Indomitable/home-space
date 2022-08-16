<script setup lang="ts">
import NewFolderAction from "./NewFolderAction.vue";
import UploadFileAction from "./UploadFileAction.vue";
</script>
<template>
    <!-- eslint-disable max-len -->
    <ul class="file-action-create-list popup">
        <li class="file-action-create-list-item file-action-create-list-item--end-group">
            <new-folder-action />
        </li>
        <li
            class="file-action-create-list-item file-action-create-list-item--start-group file-action-create-list-item--end-group"
        >
            <upload-file-action parent_id="{props.parent_id}" />
        </li>
        <li class="file-action-create-list-item file-action-create-list-item--start-group">
            <a>
                <span class="icon-outlined">description</span>
                <span>Text file</span>
            </a>
        </li>
    </ul>
</template>

<style scoped lang="scss">
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
</style>
