import { createRouter, createWebHistory, type Router } from "vue-router";
import HomeView from "@/views/HomeView.vue";
import LoginView from "@/views/auth/LoginView.vue";
import AllFiles from "@/views/home/AllFiles.vue";
import FavoriteFiles from "@/views/home/FavoriteFiles.vue";
import RecentFiles from "@/views/home/RecentFiles.vue";
import SharedFiles from "@/views/home/SharedFiles.vue";
import TrashFiles from "@/views/home/TrashFiles.vue";
import type { UserService } from "@/services/user/user-service";
import type { InjectionKey } from "vue";

export function createAppRouter(userService: UserService): Router {
    const router = createRouter({
        history: createWebHistory(import.meta.env.BASE_URL),
        routes: [
            {
                path: "/",
                redirect: { name: "files", params: { parent: "0" } },
                component: HomeView,
                children: [
                    {
                        path: "/files/:parent(\\d+)",
                        name: "files",
                        props: true,
                        component: AllFiles,
                    },
                    {
                        path: "/favorites",
                        name: "favorites",
                        component: FavoriteFiles,
                    },
                    {
                        path: "/recent",
                        name: "recent",
                        component: RecentFiles,
                    },
                    {
                        path: "/shared",
                        name: "shared",
                        component: SharedFiles,
                    },
                    {
                        path: "/trash",
                        name: "trash",
                        component: TrashFiles,
                    },
                ],
            },
            {
                path: "/about",
                name: "about",
                component: () => import("../views/AboutView.vue"),
            },
            {
                path: "/login",
                name: "login",
                component: LoginView,
                meta: {
                    guestOk: true,
                },
            },
        ],
    });

    router.beforeEach((to) => {
        if (!(to.meta.guestOk || !!userService.getLoggedUser())) {
            // this route requires auth, check if logged in
            // if not, redirect to login page.
            return {
                path: "/login",
            };
        }
    });

    return router;
}

export const routerInjectionToken: InjectionKey<Router> = Symbol("router");
