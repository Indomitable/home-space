import type { App } from "vue";
import type { Router } from "vue-router";
import { UserService, userServiceInjectionToken } from "@/services/user/user-service";
import { FormatterService, formatterServiceInjectionToken } from "@/services/formatter-service";

import { createAppRouter } from "../router";

export function provideAppServices(app: App<Element>): Router {
    const userService = new UserService();
    app.provide(userServiceInjectionToken, userService);

    app.provide(formatterServiceInjectionToken, new FormatterService());

    return createAppRouter(userService);
}
