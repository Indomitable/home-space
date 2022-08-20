<script setup lang="ts">
interface UploadFileActionEvents {
    (event: "upload-files"): void;
}

defineEmits<UploadFileActionEvents>();
</script>
<template>
    <a @click="() => $emit('upload-files')">
        <span class="icon-outlined">upload_file</span>
        <span>Upload folder or files</span>
    </a>
</template>
