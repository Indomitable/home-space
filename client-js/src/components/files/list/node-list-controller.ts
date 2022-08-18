import { reactive, computed, type ComputedRef } from "vue";
import type { Router } from "vue-router";

import type { FileNode } from "@/services/files/files-load-service";
import type { FileActionService } from "@/services/files/file-action-service";

export interface NodeState {
    selected: boolean;
}

export class NodeListController {
    nodesState: Record<number, NodeState>;
    hasNodes: ComputedRef<boolean>;
    allNodesSelected: ComputedRef<boolean>;

    constructor(public nodes: FileNode[], private fileActionService: FileActionService, private router: Router) {
        const ns = nodes.reduce((aggr, node) => {
            aggr[node.id] = {
                selected: false,
            };
            return aggr;
        }, {} as Record<number, NodeState>);
        this.nodesState = reactive(ns);
        this.hasNodes = computed(() => this.nodes.length > 0);
        this.allNodesSelected = computed(() => Object.values(this.nodesState).every(s => s.selected));
    }

    toggleNodeSelection(node: FileNode, selected: boolean): void {
        this.nodesState[node.id].selected = selected;
    }

    toggleAllNodeSelection(selected: boolean) {
        for (const node of this.nodes) {
            this.nodesState[node.id].selected = selected;
        }
    }

    async nodeTitleClicked(node: FileNode): Promise<void> {
        await this.fileActionService.open(node, this.router);
    }
}
