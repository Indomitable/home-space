import { computed, type ComputedRef, reactive, ref, type Ref } from "vue";
import type { Router } from "vue-router";

import type { FavoriteFileNode, FileNode } from "@/models/file-node";
import type { FileActionService } from "@/services/files/file-action-service";
import { SortDirection, type Sorting } from "@/models/sorting";

export interface NodeState {
    selected: boolean;
    rename: boolean;
}

export class NodeListController {
    nodes: Ref<FavoriteFileNode[]>;
    nodesState: Record<number, NodeState>;
    hasNodes: ComputedRef<boolean>;
    allNodesSelected: ComputedRef<boolean>;
    selectedNodes: ComputedRef<FileNode[]>;
    sorting: Ref<Sorting>;

    constructor(
        nodes: FavoriteFileNode[],
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
        this.sorting = ref({ sortColumn: "title", sortDirection: SortDirection.Asc });
    }

    toggleNodeSelection(node: FileNode, selected: boolean): void {
        this.nodesState[node.id].selected = selected;
        this.nodesState[node.id].rename = false;
    }

    toggleAllNodeSelection(selected: boolean) {
        for (const node of this.nodes.value) {
            this.nodesState[node.id].selected = selected;
        }
    }

    toggleNodeRename(node: FileNode, rename: boolean): void {
        this.nodesState[node.id].rename = rename;
    }

    async renameNode(node: FileNode, name: string) {
        this.toggleNodeRename(node, false);
        await this.fileActionService.renameNode(node, name);
        await this.refresh();
    }

    async nodeTitleClicked(node: FileNode): Promise<void> {
        await this.fileActionService.open(node, this.router);
    }

    async load(sorting: Sorting) {
        this.nodes.value = await this.fileActionService.loadNodes(this.parentId, sorting);
        this.sorting.value = sorting;
        for (const node of this.nodes.value) {
            if (!(node.id in this.nodesState)) {
                this.nodesState[node.id] = {
                    selected: false,
                    rename: false,
                };
            }
        }
    }

    async refresh() {
        await this.load(this.sorting.value); // reload with same sorting
    }

    async toggleNodeFavorite(node: FileNode, favorite: boolean): Promise<void> {
        await this.fileActionService.toggleNodeFavorite(node, favorite);
        await this.refresh();
    }
}
