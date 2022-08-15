<script setup lang="ts">
export interface AllFilesProps {
    parent: string; // we pass parent from url and router always pass it as string
}

const props = defineProps<AllFilesProps>();
</script>

<template>
    <div>Files. Parent is {{ props.parent }}</div>
</template>
