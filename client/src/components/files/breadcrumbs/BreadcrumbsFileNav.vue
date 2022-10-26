<script setup lang="ts">
import { breadcrumbServiceInjectionToken } from "@/services/files/breadcrumbs-service";
import { inject } from "vue";
import { useRouter } from "vue-router";
import BreadcrumbItem from "./BreadcrumbItem.vue";

export interface BreadcrumbsFileNavProps {
    parentId: number;
}
const props = defineProps<BreadcrumbsFileNavProps>();

const breadcrumbsService = inject(breadcrumbServiceInjectionToken)!;

const nodes = await breadcrumbsService.loadBreadcrumbs(props.parentId);

const router = useRouter();
function onBreadcrumbNavigate(id: number, isLast: boolean) {
    if (!isLast) {
        breadcrumbsService.navigate(router, id);
    }
}
</script>

<template>
    <nav class="breadcrumbs-nav">
        <breadcrumb-item v-if="!nodes.length" :id="0" icon="home" title="My Files" />
        <breadcrumb-item
            v-for="(node, index) in nodes"
            :key="node.id"
            :id="node.id"
            :title="node.id === 0 ? 'My Files' : node.title"
            :icon="node.id === 0 ? 'home' : ''"
            @breadcrumb-clicked="id => onBreadcrumbNavigate(id, index === nodes.length - 1)"
        />
    </nav>
</template>

<style>
.breadcrumbs-nav {
    padding: 20px 17px 30px 17px;
    display: flex;
    flex: 0 0 77px;
    white-space: nowrap;
}

@media (max-width: 900px) {
    .breadcrumbs-nav {
        padding: 10px;
        flex-basis: 50px;
    }
}
</style>
