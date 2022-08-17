import { resolveApiUrl } from "@/api/url-resolver";
import type { UserService } from "@/services/user/user-service";
import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import type { RequestInitVisitor } from "@/api/request-init-visitor";
import type { InjectionKey } from "vue";

interface FileNodeDto {
    id: number;
    user_id: number;
    title: string;
    parent_id: number;
    node_type: number;
    filesystem_path: string;
    mime_type: number;
    modified_at: string;
    node_size: number;
}

export class NodeCreateService {
    constructor(private userService: UserService) {}

    async uploadFile(parentId: number, file: File): Promise<number> {
        const url = resolveApiUrl("files", "upload-file");
        const response = await RequestBuilder.create(url)
            .setMethod(HttpMethod.PUT)
            .enhance(this.userService)
            .enhance(new UploadFileRequestEnchancer(parentId, file.name))
            .setBody(file)
            .build<FileNodeDto>()
            .execute();
        return response.id;
    }
}

class UploadFileRequestEnchancer implements RequestInitVisitor {
    constructor(private parentId: number, private fileName: string) {}
    visit(requestInit: RequestInit): void {
        const headers = requestInit.headers as Record<string, string>;
        headers["X-PARENT-ID"] = "" + this.parentId;
        headers["X-FILE-NAME"] = encodeURIComponent(this.fileName);
    }
}

export const nodeCreateServiceInjectionToken: InjectionKey<NodeCreateService> = Symbol("NodeCreateService");
