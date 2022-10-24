<script setup lang="ts">
import { inject, provide } from "vue";

import { provideFileServices } from "@/di/files.provide";

import FilesMain from "@/components/files/FilesMain.vue";
import NodeClipboard from "@/components/files/clipboard/NodeClipboard.vue";

export interface AllFilesProps {
    parent: string; // we pass parent from url and router always pass it as string
}

const props = defineProps<AllFilesProps>();
const parentId = +props.parent;

provideFileServices(provide, inject);
provide("parent-id", parentId);
</script>

<template>
    <suspense>
        <files-main :parent-id="+props.parent" />
    </suspense>
    <node-clipboard />
</template>
