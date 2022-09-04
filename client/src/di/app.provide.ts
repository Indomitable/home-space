import type { App } from "vue";
import type { Router } from "vue-router";
import { UserService, userServiceInjectionToken } from "@/services/user/user-service";
import { FormatterService, formatterServiceInjectionToken } from "@/services/formatter-service";
import { FileSystemService, fileSystemServiceInjectionToken } from "@/services/files/file-system-service";
import { FileActionService, fileActionServiceInjectionToken } from "@/services/files/file-action-service";
import { FileLoadService, fileLoadServiceInjectionToken } from "@/services/files/files-load-service";
import { ClipboardService, clipboardServiceInjectionToken } from "@/services/files/clipboard-service";

import { createAppRouter } from "@/router";

export function provideAppServices(app: App<Element>): Router {
    const userService = new UserService();
    app.provide(userServiceInjectionToken, userService);

    const formatter = new FormatterService();
    app.provide(formatterServiceInjectionToken, formatter);

    const fileSystem = new FileSystemService();
    app.provide(fileSystemServiceInjectionToken, fileSystem);

    const fileLoadService = new FileLoadService(userService, formatter);
    app.provide(fileLoadServiceInjectionToken, fileLoadService);

    const fileActionService = new FileActionService(userService, fileSystem, fileLoadService);
    app.provide(fileActionServiceInjectionToken, fileActionService);

    app.provide(clipboardServiceInjectionToken, new ClipboardService(fileActionService));

    return createAppRouter(userService);
}
