<script setup lang="ts">
import NodeList from "./NodeList.vue";

import type { NodeListController } from "@/components/files/list/node-list-controller";

export interface NodeListProps {
    controller: NodeListController;
    hasMoreFavorites: boolean;
}

const props = defineProps<NodeListProps>();
</script>

<template>
    <div class="file_view__favorite_file_list">
        <div class="file_view__favorite_file_list__header header">Favorites</div>
        <node-list :controller="props.controller" />
        <router-link v-if="hasMoreFavorites" :to="{ name: 'favorites' }" class="file-view__favorite-more-link"
            >More</router-link
        >
    </div>
</template>

<style scoped lang="scss">
.file_view__favorite_file_list__header {
    font-size: 20px;
    padding-left: 17px;
}
.file_view__favorite_file_list {
    margin-bottom: 30px;
}
.file-view__favorite-more-link {
    text-align: center;
    display: block;
    cursor: pointer;
    color: var(--link-font-color);
    font-size: 16px;
}
</style>
