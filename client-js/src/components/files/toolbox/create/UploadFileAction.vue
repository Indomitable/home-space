<script setup lang="ts">
import { inject, type Ref } from "vue";

import { fileSystemServiceInjectionToken } from "@/services/files/file-system-service";
import { nodeCreateServiceInjectionToken } from "@/services/files/node-create-service";

const fs = inject(fileSystemServiceInjectionToken)!;
const nodeCreateService = inject(nodeCreateServiceInjectionToken)!;

const parentId = inject("parent-id") as number;

async function onUpload() {
    const files = fs.loadFiles([]);
    for await (const file of files) {
        console.log(`Uploading: ${file.name}`);
        await nodeCreateService.uploadFile(parentId, file);
    }
}
</script>
<template>
    <a @click="onUpload()">
        <span class="icon-outlined">upload_file</span>
        <span>Upload folder or files</span>
    </a>
</template>
