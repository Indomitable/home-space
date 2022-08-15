<script setup lang="ts">
import { userServiceInjectionToken } from "@/auth/user-service";
import { routerInjectionToken } from "@/router";
import { inject } from "vue";

const userService = inject(userServiceInjectionToken)!;
const router = inject(routerInjectionToken)!;

const onLogout = () => {
    userService.logout();
    router.push("/login");
};
</script>

<template>
    <button class="ghost-button icon-button" v-on:click="onLogout">
        <span class="icon-filled">logout</span>
    </button>
</template>
