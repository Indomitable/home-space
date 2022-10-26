<script setup lang="ts">
import { ref, watch, watchEffect } from "vue";

interface NewFolderActionEmit {
    (event: "create-folder", name: string): void;
}

const isReadOnly = ref(true);
const folderName = ref("");

const emit = defineEmits<NewFolderActionEmit>();

function onNewFolderKeyPress(event: KeyboardEvent) {
    if (event.key === "Enter") {
        emit("create-folder", folderName.value);
    }
}

function onNewFolderKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
        isReadOnly.value = true;
    }
}

const input = ref<HTMLInputElement | null>(null);

watchEffect(() => {
    if (input.value) {
        input.value.focus();
    }
});
</script>
<template>
    <a v-on:click="isReadOnly = false">
        <span class="icon-outlined">create_new_folder</span>
        <span v-if="isReadOnly">New Folder</span>
        <input
            v-else
            ref="input"
            placeholder="Folder name"
            class="input new-folder-action-input"
            v-model="folderName"
            v-on:keypress="onNewFolderKeyPress"
            v-on:keydown="onNewFolderKeyDown"
        />
    </a>
</template>
<style>
.new-folder-action-input {
    width: 100%;
    height: 30px !important;
}
</style>
