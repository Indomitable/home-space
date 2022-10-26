<script setup lang="ts">
import { inject } from "vue";
import { useRouter } from "vue-router";

import { fileActionServiceInjectionToken } from "@/services/files/file-action-service";

import BreadcrumbsFileNav from "./breadcrumbs/BreadcrumbsFileNav.vue";
import NodeList from "./list/NodeList.vue";
import { NodeListController } from "./list/node-list-controller";
import FileActions from "./toolbox/FilesActions.vue";

export interface FilesMainProps {
    parentId: number;
}

const props = defineProps<FilesMainProps>();
const router = useRouter();
const fileActionService = inject(fileActionServiceInjectionToken)!;
const nodes = await fileActionService.loadNodes(props.parentId);

const ctrl = new NodeListController(nodes, fileActionService, router, props.parentId);
</script>

<template>
    <div class="files-main-component">
        <file-actions :parent-id="parentId" :ctrl="ctrl"></file-actions>
        <breadcrumbs-file-nav :parent-id="parentId" />
        <div class="file-view-lists-container">
            <node-list :controller="ctrl" />
        </div>
    </div>
</template>

<style>
.files-main-component {
    height: 100%;
    display: flex;
    flex-direction: column;
}

.file-view-lists-container {
    overflow: hidden;
    flex: 1;
}

@media (max-width: 900px) {
    .file-view-lists-container {
        width: 100%;
        padding: 0 10px;
    }
}
</style>
