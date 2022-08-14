<script setup lang="ts">
import { logout } from "@/auth/authentication";
import router from "@/router";

const onLogout = () => {
    logout();
    router.push("/login");
};
</script>

<template>
    <button class="ghost-button icon-button" v-on:click="onLogout">
        <span class="icon-filled">logout</span>
    </button>
</template>
