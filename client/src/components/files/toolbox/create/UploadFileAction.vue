<script setup lang="ts">
interface UploadFileActionEvents {
    (event: "upload-files"): void;
    (event: "upload-folder"): void;
}

defineEmits<UploadFileActionEvents>();

function supportDirectoryUpload() {
    return !!window.showDirectoryPicker;
}
</script>
<template>
    <a @click="() => $emit('upload-files')">
        <span class="icon-outlined">upload_file</span>
        <span>Upload files</span>
    </a>
    <a @click="() => $emit('upload-folder')" v-if="supportDirectoryUpload()">
        <span class="icon-outlined">upload_file</span>
        <span>Upload folder</span>
    </a>
</template>
