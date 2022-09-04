<script setup lang="ts">
import { inject } from "vue";

import { userServiceInjectionToken } from "@/services/user/user-service";
import TopHeader from "@/components/header/TopHeader.vue";
import LeftNavigation from "@/components/navigation/LeftNavigation.vue";

const userService = inject(userServiceInjectionToken)!;
const loggedUser = userService.getLoggedUser();
const userName = loggedUser ? loggedUser.userName : "Session expired!";
</script>

<template>
    <div class="home">
        <top-header :user-name="userName" />
        <div class="home-content">
            <aside>
                <left-navigation />
            </aside>
            <section>
                <router-view :key="$route.path" />
            </section>
        </div>
    </div>
</template>

<style scoped lang="scss">
.home {
    height: 100%;
    display: grid;
    grid-template-rows: 50px auto;
}

.home-content {
    display: grid;
    grid-template-columns: 200px auto;
}
</style>