export {};

import "vue-router";

declare module "vue-router" {
    interface RouteMeta {
        guestOk?: boolean;
    }
}
