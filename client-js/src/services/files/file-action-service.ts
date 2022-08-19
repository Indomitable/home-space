import type { InjectionKey } from "vue";
import type { Router } from "vue-router";

import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import { NodeType, type FileNode } from "@/models/file-node";

import type { UserService } from "../user/user-service";
import type { FileSystemService } from "./file-system-service";
import type { FileLoadService } from "./files-load-service";
import type { Sorting } from "@/models/sorting";

export class FileActionService {
    constructor(
        private userService: UserService,
        private fileSystem: FileSystemService,
        private fileLoadService: FileLoadService
    ) {}

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
            await this.downloadFile(node);
        }
    }

    private async downloadFile(file: FileNode): Promise<void> {
        const url = resolveApiUrl("files", "file", file.id);
        const blob = await RequestBuilder.create(url)
            .setMethod(HttpMethod.GET)
            .enhance(this.userService)
            .build("blob")
            .execute();
        await this.fileSystem.saveFile(blob, file.title, []);
    }
}

export const fileActionServiceInjectionToken: InjectionKey<FileActionService> = Symbol("FileActionService");
