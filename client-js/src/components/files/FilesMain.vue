<script setup lang="ts">
import { inject, computed } from "vue";
import { useRouter } from "vue-router";

import { fileLoadServiceInjectionToken, type FileNode } from "@/services/files/files-load-service";
import { fileActionServiceInjectionToken } from "@/services/files/file-action-service";

import FileActions from "./toolbox/FilesActions.vue";
import BreadcrumbsFileNav from "./breadcrumbs/BreadcrumbsFileNav.vue";
import FavoriteNodeList from "./list/FavoriteNodeList.vue";
import NodeList from "./list/NodeList.vue";
import { FilesMainController } from "./files-main-controller";

export interface FilesMainProps {
    parentId: number;
}

const props = defineProps<FilesMainProps>();

const fileLoadService = inject(fileLoadServiceInjectionToken)!;
const nodes = await fileLoadService.loadFileNodes(props.parentId);
const router = useRouter();
const fileActionService = inject(fileActionServiceInjectionToken)!;

const ctrl = new FilesMainController(nodes, fileActionService, router);
</script>

<template>
    <file-actions :parent-id="parentId" :ctrl="ctrl"></file-actions>
    <breadcrumbs-file-nav :parent-id="parentId" />
    <div class="file-view-lists-container">
        <favorite-node-list
            v-if="ctrl.favoritesCtrl.hasNodes.value"
            :controller="ctrl.favoritesCtrl"
            :has-more-favorites="ctrl.hasMoreFavoriteNodes"
        />
        <template v-if="ctrl.regularCtrl.hasNodes.value">
            <div v-if="ctrl.favoritesCtrl.hasNodes.value" class="file_view__file_list__header header">Files</div>
            <node-list :controller="ctrl.regularCtrl" />
        </template>
    </div>
</template>

<style scoped lang="scss">
.file_view__file_list__header {
    font-size: 20px;
    padding-left: 17px;
}

.file-view-lists-container {
    max-width: 1200px;
}
</style>
