<script setup lang="ts">
import { inject } from "vue";

import { fileLoadServiceInjectionToken, type FileNode } from "@/services/files/files-load-service";

import FileActions from "./toolbox/FilesActions.vue";
import BreadcrumbsFileNav from "./breadcrumbs/BreadcrumbsFileNav.vue";
import NodeList from "./list/NodeList.vue";

export interface FilesMainProps {
    parentId: number;
}

const props = defineProps<FilesMainProps>();

const fileLoadService = inject(fileLoadServiceInjectionToken)!;
const nodes = await fileLoadService.loadFileNodes(props.parentId);
const [regular, favorite] = nodes.reduce(
    (aggr, current) => {
        aggr[+current.isFavorite].push(current);
        return aggr;
    },
    [[], []] as [FileNode[], FileNode[]]
);
</script>

<template>
    <file-actions :parent-id="parentId" :selected-nodes="0"></file-actions>
    <breadcrumbs-file-nav :parent-id="parentId" />
    <template v-if="favorite.length > 0">
        <div class="file_view__favorite_file_list">
            <div class="file_view__favorite_file_list__header header">Favorites</div>
            <node-list :nodes="favorite" />
        </div>
    </template>
    <template v-if="regular.length > 0">
        <div v-if="favorite.length > 0" class="file_view__file_list__header header">Files</div>
        <node-list :nodes="regular" />
    </template>
</template>

<style scoped lang="scss">
.file_view__favorite_file_list {
    margin-bottom: 30px;
}

.file_view__favorite_file_list__header,
.file_view__file_list__header {
    font-size: 20px;
    padding-left: 17px;
}
</style>
