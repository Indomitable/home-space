<script setup lang="ts">
import { inject, ref, onErrorCaptured } from "vue";

import { userServiceInjectionToken } from "@/services/user/user-service";
import TopHeader from "@/components/header/TopHeader.vue";
import LeftNavigation from "@/components/navigation/LeftNavigation.vue";
import JobsNotification from "@/components/jobs/JobsNotification.vue";
import {useRouter} from "vue-router";
import {NotAuthorizedError} from "@/errors/not-authorized-error";

const userService = inject(userServiceInjectionToken)!;
const loggedUser = userService.getLoggedUser();
const userName = loggedUser ? loggedUser.userName : "Session expired!";

const leftNavVisible = ref(false);
function toggleLeftNavigation(value: boolean) {
    leftNavVisible.value = value;
}

const router = useRouter();
onErrorCaptured((error) => {
    if (error instanceof NotAuthorizedError) {
        router.push({ name: 'login' });
    }
})

</script>

<template>
    <div class="home">
        <top-header @toggle-menu="value => toggleLeftNavigation(value)" />
        <div class="home-content">
            <aside class="home-view-navigation" :class="{ 'home-view-navigation--visible': leftNavVisible }">
                <left-navigation />
            </aside>
            <section class="home-view-content">
                <router-view :key="$route.path" />
            </section>
        </div>
        <jobs-notification />
    </div>
</template>

<style lang="scss">
.home {
    height: 100%;
    width: 100%;
    display: flex;
    flex-direction: column;
}

.home-content {
    display: grid;
    grid-template-columns: 200px auto;
    width: 100%;
    height: 100%;
    overflow: hidden;
}

.home-view-content {
    width: 100%;
    height: 100%;
    overflow: hidden;
}

@media (max-width: 900px) {
    .home-content {
        display: block;
    }
    .home-view-navigation {
        position: absolute;
        width: 200px;
        height: 100%;
        background: var(--background-color);
        border-right: 1px solid var(--border-color);
        box-shadow: 2px 2px 6px 2px #443d3d;
        z-index: 10;
        left: -210px;
        transition: cubic-bezier(1, 0, 0, 1) 0.5s left;
    }
    .home-view-navigation.home-view-navigation--visible {
        left: 0px;
    }
}
</style>
