import { reactive, computed, type ComputedRef, ref, type Ref } from "vue";
import type { Router } from "vue-router";

import type { FileNode } from "@/models/file-node";
import type { FileActionService } from "@/services/files/file-action-service";
import { SortDirection, type Sorting } from "@/models/sorting";

export interface NodeState {
    selected: boolean;
    rename: boolean;
}

export class NodeListController {
    nodes: Ref<FileNode[]>;
    nodesState: Record<number, NodeState>;
    hasNodes: ComputedRef<boolean>;
    allNodesSelected: ComputedRef<boolean>;
    selectedNodes: ComputedRef<FileNode[]>;
    sorting: Ref<Sorting>;

    constructor(
        nodes: FileNode[],
        private fileActionService: FileActionService,
        private router: Router,
        private parentId: number
    ) {
        this.nodes = ref(nodes);
        this.nodesState = reactive(
            nodes.reduce((aggr, node) => {
                aggr[node.id] = {
                    selected: false,
                    rename: false,
                };
                return aggr;
            }, {} as Record<number, NodeState>)
        );
        this.hasNodes = computed(() => this.nodes.value.length > 0);
        this.allNodesSelected = computed(() => Object.values(this.nodesState).every(s => s.selected));
        this.selectedNodes = computed(() => this.nodes.value.filter(n => this.nodesState[n.id].selected));
        this.sorting = ref({ columnName: "title", direction: SortDirection.Asc });
    }

    toggleNodeSelection(node: FileNode, selected: boolean): void {
        this.nodesState[node.id].selected = selected;
        this.nodesState[node.id].rename = false;
    }

    toggleAllNodeSelection(selected: boolean) {
        for (const node of this.nodes.value) {
            this.nodesState[node.id].selected = selected;
            this.nodesState[node.id].rename = false;
        }
    }

    toggleNodeRename(node: FileNode, rename: boolean): void {
        this.nodesState[node.id].rename = rename;
    }

    async nodeTitleClicked(node: FileNode): Promise<void> {
        await this.fileActionService.open(node, this.router);
    }

    async sortNodes(sorting: Sorting) {
        const nodes = await this.fileActionService.loadNodes(this.parentId, sorting);
        this.nodes.value = nodes;
        this.sorting.value = sorting;
        for (const node of this.nodes.value) {
            if (!(node.id in this.nodesState)) {
                this.nodesState[node.id] = {
                    selected: false,
                    rename: false,
                };
            } else {
                this.nodesState[node.id].rename = false;
            }
        }
    }
}
