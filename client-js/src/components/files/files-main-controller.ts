import { computed, type ComputedRef } from "vue";
import type { Router } from "vue-router";

import type { FileActionService } from "@/services/files/file-action-service";
import type { FileNode } from "@/services/files/files-load-service";

import { NodeListController } from "./list/node-list-controller";

export class FilesMainController {
    regularCtrl: NodeListController;
    favoritesCtrl: NodeListController;
    hasMoreFavoriteNodes = false;
    allSelectedNodes: ComputedRef<FileNode[]>;

    constructor(private nodes: FileNode[], private fileActionService: FileActionService, private router: Router) {
        const { regular, favorites } = nodes.reduce(
            (aggr, current) => {
                if (current.isFavorite) {
                    if (aggr.favorites.length < 4) {
                        aggr.favorites.push(current);
                    } else {
                        this.hasMoreFavoriteNodes = true;
                    }
                }
                aggr.regular.push(current);
                return aggr;
            },
            { regular: [], favorites: [] } as { regular: FileNode[]; favorites: FileNode[] }
        );
        this.regularCtrl = new NodeListController(regular, fileActionService, router);
        this.favoritesCtrl = new NodeListController(favorites, fileActionService, router);
        this.allSelectedNodes = computed(() => [
            ...this.regularCtrl.selectedNodes.value,
            ...this.favoritesCtrl.selectedNodes.value,
        ]);
    }

    toggleSelectedNodeRename(rename: boolean): void {
        const nodeSelected = this.allSelectedNodes;
        if (nodeSelected.value.length === 1) {
            const node = nodeSelected.value[0];
            this.favoritesCtrl.toggleNodeRename(node, rename);
        }
    }
}
