import { breadcrumbServiceInjectionToken, BreadcrumbsService } from "@/services/files/breadcrumbs-service";
import { FileActionService, fileActionServiceInjectionToken } from "@/services/files/file-action-service";
import { FileSystemService, fileSystemServiceInjectionToken } from "@/services/files/file-system-service";
import { FileLoadService, fileLoadServiceInjectionToken } from "@/services/files/files-load-service";
import { NodeCreateService, nodeCreateServiceInjectionToken } from "@/services/files/node-create-service";
import { formatterServiceInjectionToken } from "@/services/formatter-service";
import { userServiceInjectionToken } from "@/services/user/user-service";
import type { Provider, Injector } from ".";

export function provideFileServices(provide: Provider, inject: Injector): void {
    const userService = inject(userServiceInjectionToken)!;

    const fileSystem = new FileSystemService();
    provide(fileSystemServiceInjectionToken, fileSystem);

    const fileActionService = new FileActionService(userService, fileSystem);
    provide(fileActionServiceInjectionToken, fileActionService);
    provide(breadcrumbServiceInjectionToken, new BreadcrumbsService(userService, fileActionService));

    const formatter = inject(formatterServiceInjectionToken)!;
    provide(fileLoadServiceInjectionToken, new FileLoadService(userService, formatter));

    provide(nodeCreateServiceInjectionToken, new NodeCreateService(userService));
}
