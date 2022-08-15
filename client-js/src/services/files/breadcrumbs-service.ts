import { HttpMethod, RequestBuilder } from "@/api/request-builder";
import { resolveApiUrl } from "@/api/url-resolver";
import type { InjectionKey } from "vue";
import type { UserService } from "../user/user-service";

interface ParentNode {
    id: number;
    title: string;
}

export class BreadcrumbsService {
    constructor(private userService: UserService) {}

    async loadBreadcrumbs(parentId: number): Promise<ParentNode[]> {
        const parentsUrl = resolveApiUrl("files", "parents");
        const response = RequestBuilder.create(`${parentsUrl}/${parentId}`)
            .setMethod(HttpMethod.GET)
            .enhance(this.userService)
            .build<ParentNode[]>()
            .execute();
        return response;
    }
}

export const breadcrumbServiceInjectionToken: InjectionKey<BreadcrumbsService> = Symbol("BreadcrumbsService");
