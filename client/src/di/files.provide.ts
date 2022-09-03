import { breadcrumbServiceInjectionToken, BreadcrumbsService } from "@/services/files/breadcrumbs-service";
import { fileActionServiceInjectionToken } from "@/services/files/file-action-service";
import { userServiceInjectionToken } from "@/services/user/user-service";
import type { Provider, Injector } from ".";

export function provideFileServices(provide: Provider, inject: Injector): void {
    const userService = inject(userServiceInjectionToken)!;
    const fileActionService = inject(fileActionServiceInjectionToken)!;

    provide(breadcrumbServiceInjectionToken, new BreadcrumbsService(userService, fileActionService));
}
