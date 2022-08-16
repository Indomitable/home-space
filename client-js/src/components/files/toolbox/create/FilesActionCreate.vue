<script setup lang="ts">
import { ref } from "vue";
import FilesActionCreateList from "./FilesActionCreateList.vue";

const listVisible = ref(false);
</script>
<template>
    <button class="file-action-create ghost-button" v-on:click="listVisible = !listVisible">
        <span class="icon-outlined">note_add</span>
        <span>Create</span>
        <span class="icon-filled">arrow_drop_down</span>
    </button>
    <files-action-create-list v-if="listVisible" />
</template>
<style scoped lang="scss">
.file-action-create {
    padding: 2px 7px;
    display: flex;
    column-gap: 5px;
    align-items: center;
    font-size: 1rem;
}
</style>
