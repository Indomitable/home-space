import { routerInjectionToken } from "@/router";
import { breadcrumbServiceInjectionToken, BreadcrumbsService } from "@/services/files/breadcrumbs-service";
import { FileSystemService, fileSystemServiceInjectionToken } from "@/services/files/file-system-service";
import { FileLoadService, fileLoadServiceInjectionToken } from "@/services/files/files-load-service";
import { NodeCreateService, nodeCreateServiceInjectionToken } from "@/services/files/node-create-service";
import { formatterServiceInjectionToken } from "@/services/formatter-service";
import { userServiceInjectionToken } from "@/services/user/user-service";
import type { Provider, Injector } from ".";

export function provideFileServices(provide: Provider, inject: Injector): void {
    const userService = inject(userServiceInjectionToken)!;
    const router = inject(routerInjectionToken)!;

    provide(breadcrumbServiceInjectionToken, new BreadcrumbsService(userService, router));

    const formatter = inject(formatterServiceInjectionToken)!;
    provide(fileLoadServiceInjectionToken, new FileLoadService(userService, formatter));

    provide(fileSystemServiceInjectionToken, new FileSystemService());
    provide(nodeCreateServiceInjectionToken, new NodeCreateService(userService));
}
