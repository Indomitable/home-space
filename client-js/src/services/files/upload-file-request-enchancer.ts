import type { RequestInitVisitor } from "@/api/request-init-visitor";

export class UploadFileRequestEnchancer implements RequestInitVisitor {
    constructor(private parentId: number, private fileName: string) {}
    visit(requestInit: RequestInit): void {
        const headers = requestInit.headers as Record<string, string>;
        headers["X-PARENT-ID"] = "" + this.parentId;
        headers["X-FILE-NAME"] = encodeURIComponent(this.fileName);
    }
}
