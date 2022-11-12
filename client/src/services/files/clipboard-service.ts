import type { FileNode } from "@/models/file-node";
import { computed, type ComputedRef, type InjectionKey, reactive } from "vue";
import type { FileActionService } from "./file-action-service";

enum ClipboardOperation {
    Cut = 1,
    Copy = 2,
}

interface ClipboardState {
    operation: ClipboardOperation;
    parentId: number;
    items: FileNode[];
    itemsIndex: { [id: number]: FileNode };
}

class ClipboardService {
    private state: ClipboardState;
    public hasItems: ComputedRef<boolean>;

    constructor(private fileActionsService: FileActionService) {        
        this.state = reactive(this.loadInitialState());
        this.hasItems = computed(() => {
            return Object.keys(this.state.items).length > 0;
        });
    }

    /*
     * Add nodes from one parent to the clipboard.
     */
    public addToClipboard(parentId: number, items: FileNode[], operation: ClipboardOperation): boolean {
        if (this.hasItems.value) {
            return false;
        }
        this.state.parentId = parentId;
        this.state.items = items;
        const itemsIndex = items.reduce((aggr, node) => {
            aggr[node.id] = node;
            return aggr;
        }, {} as { [id: number]: FileNode });
        this.state.itemsIndex = itemsIndex;
        this.state.operation = operation;
        this.saveState(parentId, items, itemsIndex, operation);
        return true;
    }

    public async paste(parentId: number) {
        await this.fileActionsService.pasteNodes(this.state.items, this.state.operation, parentId);
        this.clear();
    }

    public clear() {
        this.state.items = [];
        this.state.itemsIndex = {};
        this.state.parentId = 0;
        delete localStorage["clipboard"];
    }

    public get parentId(): number {
        return this.state.parentId;
    }

    public get items(): FileNode[] {
        return this.state.items;
    }

    public get itemsIndex(): { [id: number]: FileNode } {
        return this.state.itemsIndex;
    }

    public get operation(): ClipboardOperation {
        return this.state.operation;
    }
    
    private saveState(parentId: number, items: FileNode[], itemsIndex: { [p: number]: FileNode }, operation: ClipboardOperation) {
        localStorage["clipboard"] = JSON.stringify({
            parentId,
            items,
            itemsIndex,
            operation
        } as ClipboardState);
    }
    
    private loadInitialState(): ClipboardState {
        const clipboard = localStorage["clipboard"];
        return  clipboard && JSON.parse(clipboard) || {
            operation: ClipboardOperation.Copy,
            parentId: 0,
            items: [],
            itemsIndex: {},
        };
    }
}

const clipboardServiceInjectionToken: InjectionKey<ClipboardService> = Symbol("ClipboardService");

export { clipboardServiceInjectionToken, ClipboardService, ClipboardOperation };
