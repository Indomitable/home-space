import { breadcrumbServiceInjectionToken, BreadcrumbsService } from "@/services/files/breadcrumbs-service";
import { FileActionService, fileActionServiceInjectionToken } from "@/services/files/file-action-service";
import { FileSystemService, fileSystemServiceInjectionToken } from "@/services/files/file-system-service";
import { FileLoadService, fileLoadServiceInjectionToken } from "@/services/files/files-load-service";
import { formatterServiceInjectionToken } from "@/services/formatter-service";
import { userServiceInjectionToken } from "@/services/user/user-service";
import type { Provider, Injector } from ".";

export function provideFileServices(provide: Provider, inject: Injector): void {
    const userService = inject(userServiceInjectionToken)!;

    const fileSystem = new FileSystemService();
    provide(fileSystemServiceInjectionToken, fileSystem);

    const formatter = inject(formatterServiceInjectionToken)!;
    const fileLoadService = new FileLoadService(userService, formatter);
    provide(fileLoadServiceInjectionToken, fileLoadService);

    const fileActionService = new FileActionService(userService, fileSystem, fileLoadService);
    provide(fileActionServiceInjectionToken, fileActionService);

    provide(breadcrumbServiceInjectionToken, new BreadcrumbsService(userService, fileActionService));
}
