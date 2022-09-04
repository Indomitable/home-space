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
    modifiedAtDisplay: string;
    nodeSize: number;
    nodeSizeHuman: string;
    nodeVersion: number;
    isFavorite: boolean;
}
