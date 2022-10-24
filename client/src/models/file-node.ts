export enum NodeType {
    Folder = "Folder",
    File = "File",
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
}

export interface FavoriteFileNode extends FileNode {
    isFavorite: boolean;
}
