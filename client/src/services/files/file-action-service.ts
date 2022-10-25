import type { InjectionKey } from "vue";
import type { Router } from "vue-router";

import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import { type FileNode, NodeType } from "@/models/file-node";
import type { Sorting } from "@/models/sorting";

import type { UserService } from "../user/user-service";
import type { FileSystemService } from "./file-system-service";
import type { FileLoadService } from "./files-load-service";
import { UploadFileRequestEnhancer } from "./upload-file-request-enhancer";
import type { ClipboardOperation } from "./clipboard-service";
import type { FileNodeResponse } from "@/dto/file-node-response";

export class FileActionService {
    constructor(private userService: UserService, private fileSystem: FileSystemService, private fileLoadService: FileLoadService) {}

    loadNodes(parentId: number, sorting?: Sorting) {
        return this.fileLoadService.loadFileNodes(parentId, sorting);
    }

    async navigateFolder(router: Router, id: number): Promise<void> {
        await router.push({ name: "files", params: { parent: id } });
    }

    async open(node: FileNode, router: Router): Promise<void> {
        if (node.nodeType === NodeType.Folder) {
            await this.navigateFolder(router, node.id);
        } else {
            await this.downloadNodes([node]);
        }
    }

    async toggleNodeFavorite(node: FileNode, favorite: boolean): Promise<void> {
        const url = resolveApiUrl("favorites", "toggle");
        await RequestBuilder.create(url)
            .setMethod(HttpMethod.POST)
            .enhance(this.userService)
            .setJsonBody({ id: node.id, favorite })
            .build("")
            .execute();
    }

    createFolder(parentId: number, folderName: string): Promise<{ id: number }> {
        const url = resolveApiUrl("files", "folder");
        return RequestBuilder.create(url)
            .setMethod(HttpMethod.PUT)
            .enhance(this.userService)
            .setJsonBody({ parentId: parentId, name: folderName })
            .build<{ id: number }>("json")
            .execute();
    }

    async uploadFile(parentId: number, file: File): Promise<number> {
        const url = resolveApiUrl("files", "file");
        const formData = new FormData();
        formData.append("parentId", "" + parentId);
        formData.append("file", file, file.name);
        const response = await RequestBuilder.create(url)
            .setMethod(HttpMethod.PUT)
            .enhance(this.userService)
            .enhance(new UploadFileRequestEnhancer(parentId))
            .setBody(formData)
            .build<FileNodeResponse>()
            .execute();
        return response.id;
    }

    async deleteNode(nodeId: number): Promise<void> {
        const url = resolveApiUrl("trash", "delete", nodeId);
        await RequestBuilder.create(url).setMethod(HttpMethod.DELETE).enhance(this.userService).build("").execute();
    }

    async downloadNodes(nodes: FileNode[]): Promise<void> {
        function getSaveFileName(): string {
            if (nodes.length === 1) {
                const node = nodes[0];
                return node.nodeType === NodeType.File ? node.title : `${node.title}.zip`;
            }
            return "archive.zip";
        }

        const url = resolveApiUrl("files", "download");
        const query = new URLSearchParams(nodes.map(n => ["id", "" + n.id]));
        const data = await RequestBuilder.create(`${url}?${query.toString()}`)
            .setMethod(HttpMethod.GET)
            .enhance(this.userService)
            .build("stream")
            .execute();
        const saveFileName = getSaveFileName();
        if (data.stream != null) {
            await this.fileSystem.saveFileStream({ stream: data.stream, length: data.length }, saveFileName, []);
        }
    }

    async pasteNodes(nodes: FileNode[], operation: ClipboardOperation, destination: number): Promise<void> {
        const url = resolveApiUrl("files", "paste", destination);
        await RequestBuilder.create(url)
            .setMethod(HttpMethod.POST)
            .enhance(this.userService)
            .setJsonBody({
                nodes: nodes.map(n => n.id),
                operation,
            })
            .build("")
            .execute();
    }

    async renameNode(node: FileNode, name: string): Promise<void> {
        const url = resolveApiUrl("files", "rename");
        await RequestBuilder.create(url)
            .setMethod(HttpMethod.POST)
            .enhance(this.userService)
            .setJsonBody({
                id: node.id,
                name: name,
            })
            .build("")
            .execute();
    }
}

export const fileActionServiceInjectionToken: InjectionKey<FileActionService> = Symbol("FileActionService");
