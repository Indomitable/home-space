<script setup lang="ts">
import { ref, defineEmits } from "vue";

export interface NewFolderActionEmit {
    (event: "new-folder-name", name: string): void;
}

const isReadOnly = ref(true);
const folderName = ref("");

const emit = defineEmits<NewFolderActionEmit>();

function onNewFolderKeyPress(event: KeyboardEvent) {
    if (event.code === "Enter") {
        emit("new-folder-name", folderName.value);
    }
}

function onNewFolderKeyDown(event: KeyboardEvent) {
    if (event.code === "Escape") {
        isReadOnly.value = true;
    }
}
</script>
<template>
    <a v-on:click="isReadOnly = false">
        <span class="icon-outlined">create_new_folder</span>
        <span v-if="isReadOnly">New Folder</span>
        <input
            v-else
            placeholder="Folder name"
            class="input new-folder-action-input"
            v-model="folderName"
            v-on:keypress="onNewFolderKeyPress"
            v-on:keydown="onNewFolderKeyDown"
        />
    </a>
</template>
<style scopped lang="scss">
.new-folder-action-input {
    width: 100%;
    height: 30px !important;
}
</style>
