<script setup lang="ts">
import { userServiceInjectionToken } from "@/services/user/user-service";
import { inject } from "vue";
import { useRouter } from "vue-router";

const userService = inject(userServiceInjectionToken)!;
const router = useRouter();

const onLogout = () => {
    userService.logout();
    router.push("/login");
};
</script>

<template>
    <button class="ghost-button icon-button" v-on:click="onLogout">
        <span class="icon-filled">logout</span>
        <span>Logout</span>
    </button>
</template>
