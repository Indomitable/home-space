import type { App } from "vue";
import type { Router } from "vue-router";
import { UserService, userServiceInjectionToken } from "@/services/user/user-service";
import { createAppRouter, routerInjectionToken } from "../router";

export function provideApp(app: App<Element>): Router {
    const userService = new UserService();
    app.provide(userServiceInjectionToken, userService);

    const router = createAppRouter(userService);
    app.provide(routerInjectionToken, router);

    return router;
}
