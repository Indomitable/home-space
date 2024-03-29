export interface FileNodeResponse {
    id: number;
    userId: number;
    title: string;
    parentId?: number;
    nodeType: string;
    fileSystemPath: string;
    mimeType: string;
    modifiedAt: string;
    size: number;
    version: number;
}
