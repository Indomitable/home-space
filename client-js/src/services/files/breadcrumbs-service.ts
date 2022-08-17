import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import type { InjectionKey } from "vue";
import type { Router } from "vue-router";
import { resolveApiUrl } from "@/api/url-resolver";
import type { UserService } from "@/services/user/user-service";
import type { FileActionService } from "./file-action-service";

interface ParentNode {
    id: number;
    title: string;
}

export class BreadcrumbsService {
    constructor(private userService: UserService, private fileActionService: FileActionService) {}

    async loadBreadcrumbs(parentId: number): Promise<ParentNode[]> {
        const url = resolveApiUrl("files", "parents", parentId);
        const response = RequestBuilder.create(url)
            .setMethod(HttpMethod.GET)
            .enhance(this.userService)
            .build<ParentNode[]>()
            .execute();
        return response;
    }

    navigate(router: Router, id: number): void {
        this.fileActionService.navigateFolder(router, id);
    }
}

export const breadcrumbServiceInjectionToken: InjectionKey<BreadcrumbsService> = Symbol("BreadcrumbsService");
