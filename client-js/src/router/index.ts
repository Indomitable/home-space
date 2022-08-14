import { createRouter, createWebHistory } from "vue-router";
import { isAuthenticated } from "../auth/authentication";
import HomeView from "@/views/HomeView.vue";
import LoginView from "@/views/auth/LoginView.vue";
import AllFiles from "@/views/home/AllFiles.vue";
import FavoriteFiles from "@/views/home/FavoriteFiles.vue";
import RecentFiles from "@/views/home/RecentFiles.vue";
import SharedFiles from "@/views/home/SharedFiles.vue";
import TrashFiles from "@/views/home/TrashFiles.vue";

const router = createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: "/",
            redirect: "files",
            component: HomeView,
            children: [
                {
                    path: "/files",
                    name: "files",
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
    if (!(to.meta.guestOk || isAuthenticated())) {
        // this route requires auth, check if logged in
        // if not, redirect to login page.
        return {
            path: "/login",
        };
    }
});

export default router;
