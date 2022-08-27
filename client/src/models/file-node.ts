export enum NodeType {
    File,
    Folder,
}

export interface FileNode {
    id: number;
    title: string;
    parentId?: number;
    nodeType: NodeType;
    mimeType: string;
    modifiedAt: Date;
    nodeSize: number;
    nodeSizeHuman: string;
    isFavorite: boolean;
}
