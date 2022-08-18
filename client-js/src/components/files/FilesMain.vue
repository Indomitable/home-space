<script setup lang="ts">
import { inject, computed } from "vue";
import { useRouter } from "vue-router";

import { fileLoadServiceInjectionToken, type FileNode } from "@/services/files/files-load-service";
import { fileActionServiceInjectionToken } from "@/services/files/file-action-service";

import FileActions from "./toolbox/FilesActions.vue";
import BreadcrumbsFileNav from "./breadcrumbs/BreadcrumbsFileNav.vue";
import FavoriteNodeList from "./list/FavoriteNodeList.vue";
import NodeList from "./list/NodeList.vue";
import { NodeListController } from "./list/node-list-controller";

export interface FilesMainProps {
    parentId: number;
}

const props = defineProps<FilesMainProps>();

const fileLoadService = inject(fileLoadServiceInjectionToken)!;
const nodes = await fileLoadService.loadFileNodes(props.parentId);
let hasMoreFavoriteNodes = false;
const { regular, favorites } = nodes.reduce(
    (aggr, current) => {
        if (current.isFavorite) {
            if (aggr.favorites.length < 4) {
                aggr.favorites.push(current);
            } else {
                hasMoreFavoriteNodes = true;
            }
        }
        aggr.regular.push(current);
        return aggr;
    },
    { regular: [], favorites: [] } as { regular: FileNode[]; favorites: FileNode[] }
);

const router = useRouter();
const fileActionService = inject(fileActionServiceInjectionToken)!;

const regularNodesController = new NodeListController(regular, fileActionService, router);
const favoritesNodesController = new NodeListController(favorites, fileActionService, router);
const allSelectedNodes = computed(() => [
    ...regularNodesController.selectedNodes.value,
    ...favoritesNodesController.selectedNodes.value,
]);
</script>

<template>
    <file-actions :parent-id="parentId" :selected-nodes="allSelectedNodes.length"></file-actions>
    <breadcrumbs-file-nav :parent-id="parentId" />
    <favorite-node-list
        v-if="favoritesNodesController.hasNodes.value"
        :controller="favoritesNodesController"
        :has-more-favorites="hasMoreFavoriteNodes"
    />
    <template v-if="regularNodesController.hasNodes.value">
        <div v-if="favoritesNodesController.hasNodes.value" class="file_view__file_list__header header">Files</div>
        <node-list :controller="regularNodesController" />
    </template>
</template>

<style scoped lang="scss">
.file_view__file_list__header {
    font-size: 20px;
    padding-left: 17px;
}
</style>
