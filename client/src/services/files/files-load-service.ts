import type { InjectionKey } from "vue";

import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import { NodeType, type FileNode } from "@/models/file-node";

import type { FormatterService } from "../formatter-service";
import type { UserService } from "../user/user-service";
import { SortDirection, type Sorting } from "@/models/sorting";

interface FileNodeDto {
    id: number;
    title: string;
    parent_id?: number;
    node_type: number;
    mime_type: string;
    modified_at: string;
    node_size: number;
    node_version: number;
    is_favorite: boolean;
}

export class FileLoadService {
    constructor(private userService: UserService, private formatter: FormatterService) {}

    async loadFileNodes(
        parentId: number,
        sorting: Sorting = { columnName: "title", direction: SortDirection.Asc }
    ): Promise<FileNode[]> {
        const url = resolveApiUrl("files", "nodes", parentId);
        const query = new URLSearchParams({
            column_name: sorting.columnName,
            direction: sorting.direction,
        });
        const sortedUrl = `${url}?${query.toString()}`;
        const response = await RequestBuilder.create(sortedUrl)
            .setMethod(HttpMethod.GET)
            .enhance(this.userService)
            .build<FileNodeDto[]>()
            .execute();
        return response.map(n => this.mapNode(n));
    }

    private mapNode(node: FileNodeDto): FileNode {
        return {
            id: node.id,
            title: node.title,
            parentId: node.parent_id,
            nodeType: node.node_type === 0 ? NodeType.Folder : NodeType.File,
            mimeType: node.mime_type,
            modifiedAt: new Date(node.modified_at),
            modifiedAtDisplay: this.formatter.formatDate(new Date(node.modified_at)),
            nodeSize: node.node_size,
            nodeSizeHuman: this.formatter.formatSize(node.node_size),
            nodeVersion: node.node_version,
            isFavorite: node.is_favorite,
        };
    }
}

export const fileLoadServiceInjectionToken: InjectionKey<FileLoadService> = Symbol("FileLoadService");
