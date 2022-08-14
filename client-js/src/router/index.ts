import { createRouter, createWebHistory } from "vue-router";
import { isAuthenticated } from "../auth/authentication";
import HomeView from "../views/HomeView.vue";
import LoginView from "../views/auth/LoginView.vue";

const router = createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: "/",
            name: "home",
            component: HomeView,
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
