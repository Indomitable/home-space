import { breadcrumbServiceInjectionToken, BreadcrumbsService } from "@/services/files/breadcrumbs-service";
import { userServiceInjectionToken } from "@/services/user/user-service";
import type { Provider, Injector } from ".";

export function provideFileServices(provide: Provider, inject: Injector): void {
    const userService = inject(userServiceInjectionToken)!;

    const breadcrumbsService = new BreadcrumbsService(userService);
    provide(breadcrumbServiceInjectionToken, breadcrumbsService);
}
