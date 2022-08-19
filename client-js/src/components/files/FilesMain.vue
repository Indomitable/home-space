<script setup lang="ts">
import { inject } from "vue";
import { useRouter } from "vue-router";

import { fileLoadServiceInjectionToken } from "@/services/files/files-load-service";
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
const router = useRouter();
const fileActionService = inject(fileActionServiceInjectionToken)!;

const ctrl = new NodeListController(nodes, fileActionService, router);
</script>

<template>
    <file-actions :parent-id="parentId" :ctrl="ctrl"></file-actions>
    <breadcrumbs-file-nav :parent-id="parentId" />
    <div class="file-view-lists-container">
        <node-list :controller="ctrl" />
    </div>
</template>

<style scoped lang="scss">
.file-view-lists-container {
    max-width: 1200px;
}
</style>
