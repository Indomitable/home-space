import type { InjectionKey } from "vue";

import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import type { FavoriteFileNode, NodeType } from "@/models/file-node";
import { SortDirection, type Sorting } from "@/models/sorting";
import type { DisplayFileNode } from "@/dto/display-file-node";

import type { FormatterService } from "../formatter-service";
import type { UserService } from "../user/user-service";
import type { PagedResult } from "@/dto/paged-result";

export class FileLoadService {
    constructor(private userService: UserService, private formatter: FormatterService) {}

    async loadFileNodes(
        parentId: number,
        sorting: Sorting = { sortColumn: "title", sortDirection: SortDirection.Asc }
    ): Promise<FavoriteFileNode[]> {
        const url = resolveApiUrl("files", "");
        const query = new URLSearchParams({
            page: "1",
            pageSize: "100",
            parentId: "" + parentId,
            sortColumn: sorting.sortColumn,
            sortDirection: sorting.sortDirection,
        });
        const sortedUrl = `${url}?${query.toString()}`;
        const response = await RequestBuilder.create(sortedUrl)
            .setMethod(HttpMethod.GET)
            .enhance(this.userService)
            .build<PagedResult<DisplayFileNode>>()
            .execute();
        return response.pageData.map(n => this.mapNode(n));
    }

    private mapNode(node: DisplayFileNode): FavoriteFileNode {
        return {
            id: node.id,
            title: node.title,
            parentId: node.parentId,
            nodeType: <NodeType>node.nodeType,
            mimeType: node.mimeType,
            modifiedAt: new Date(node.modifiedAt),
            modifiedAtDisplay: this.formatter.formatDate(new Date(node.modifiedAt)),
            nodeSize: node.size,
            nodeSizeHuman: this.formatter.formatSize(node.size),
            nodeVersion: node.version,
            isFavorite: node.isFavorite,
        };
    }
}

export const fileLoadServiceInjectionToken: InjectionKey<FileLoadService> = Symbol("FileLoadService");
