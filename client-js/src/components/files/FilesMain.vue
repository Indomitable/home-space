<script setup lang="ts">
import { inject } from "vue";
import { useRouter } from "vue-router";

import { fileLoadServiceInjectionToken, type FileNode } from "@/services/files/files-load-service";
import { fileActionServiceInjectionToken } from "@/services/files/file-action-service";

import FileActions from "./toolbox/FilesActions.vue";
import BreadcrumbsFileNav from "./breadcrumbs/BreadcrumbsFileNav.vue";
import NodeList from "./list/NodeList.vue";
import { NodeListController } from "./list/node-list-controller";

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

const router = useRouter();
const fileActionService = inject(fileActionServiceInjectionToken)!;

const regularNodesController = new NodeListController(regular, fileActionService, router);
const favoritesNodesController = new NodeListController(favorite, fileActionService, router);
</script>

<template>
    <file-actions :parent-id="parentId" :selected-nodes="0"></file-actions>
    <breadcrumbs-file-nav :parent-id="parentId" />
    <template v-if="favoritesNodesController.hasNodes.value">
        <div class="file_view__favorite_file_list">
            <div class="file_view__favorite_file_list__header header">Favorites</div>
            <node-list :controller="favoritesNodesController" />
        </div>
    </template>
    <template v-if="regularNodesController.hasNodes.value">
        <div v-if="favoritesNodesController.hasNodes.value" class="file_view__file_list__header header">Files</div>
        <node-list :controller="regularNodesController" />
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
