<script setup lang="ts">
import { inject, type Ref } from "vue";
import type { FileNode } from "@/models/file-node";
import { ClipboardOperation, clipboardServiceInjectionToken } from "@/services/files/clipboard-service";

import { CopyStrategyChooser } from "./copy-stragety";

export interface CopyActionProps {
    operation: ClipboardOperation;
    selectedNodes: Ref<FileNode[]>;
}
const props = defineProps<CopyActionProps>();
const clipboardService = inject(clipboardServiceInjectionToken)!;
const strategy = CopyStrategyChooser.get(props.operation);
const parentId = inject("parent-id") as number;

function onActivate() {
    clipboardService.addToClipboard(parentId, props.selectedNodes.value, props.operation);
}
</script>

<template>
    <button :class="$attrs.class" class="icon-button ghost-button" @click="onActivate" v-if="!clipboardService.hasItems.value">
        <span class="icon-outlined">{{ strategy.buttonIcon }}</span>
        {{ strategy.buttonTitle }}
    </button>
</template>
