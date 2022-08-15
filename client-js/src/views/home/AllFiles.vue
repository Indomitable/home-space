<script setup lang="ts">
import FileActions from "@/components/files/toolbox/FilesActions.vue";

export interface AllFilesProps {
    parent: string; // we pass parent from url and router always pass it as string
}

const props = defineProps<AllFilesProps>();
</script>

<template>
    <file-actions :parent-id="+props.parent" :selected-nodes="0"></file-actions>
</template>
