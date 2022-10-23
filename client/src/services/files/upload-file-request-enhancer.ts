import type { RequestInitVisitor } from "@/api/request-init-visitor";

export class UploadFileRequestEnhancer implements RequestInitVisitor {
    constructor(private parentId: number) {}
    visit(requestInit: RequestInit): void {
        const headers = requestInit.headers as Record<string, string>;
        headers["X-PARENT-ID"] = "" + this.parentId;
    }
}
