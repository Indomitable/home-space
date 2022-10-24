<script setup lang="ts">
import { inject, ref } from "vue";

import { userServiceInjectionToken } from "@/services/user/user-service";
import TopHeader from "@/components/header/TopHeader.vue";
import LeftNavigation from "@/components/navigation/LeftNavigation.vue";

const userService = inject(userServiceInjectionToken)!;
const loggedUser = userService.getLoggedUser();
const userName = loggedUser ? loggedUser.userName : "Session expired!";

const leftNavVisible = ref(false);
function toggleLeftNavigation(value: boolean) {
    leftNavVisible.value = value;
}
</script>

<template>
    <div class="home">
        <top-header :user-name="userName" @toggle-menu="value => toggleLeftNavigation(value)" />
        <div class="home-content">
            <aside class="home-view-navigation" :class="{ 'home-view-navigation--visible': leftNavVisible }">
                <left-navigation />
            </aside>
            <section class="home-view-content">
                <router-view :key="$route.path" />
            </section>
        </div>
    </div>
</template>

<style lang="scss">
.home {
    height: 100%;
    width: 100%;
    display: grid;
    grid-template-rows: 50px auto;
}

.home-content {
    display: grid;
    grid-template-columns: 200px auto;
    width: 100%;
}

.home-view-content {
    overflow-x: auto;
}

@media (max-width: 900px) {
    .home-content {
        display: block;
        overflow: hidden;
    }
    .home-view-navigation {
        position: absolute;
        width: 200px;
        height: 100%;
        background: var(--background-color);
        z-index: 10;
        left: -200px;
        transition: cubic-bezier(1, 0, 0, 1) 0.5s left;
    }
    .home-view-navigation.home-view-navigation--visible {
        left: 0px;
    }
    .home-view-content {
        width: 100%;
        overflow: hidden;
    }
}
</style>
