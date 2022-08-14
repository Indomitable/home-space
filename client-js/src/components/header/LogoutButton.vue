<script lang="ts">
import { logout } from "@/auth/authentication";
import router from "@/router";
import { defineComponent } from "vue";

export default defineComponent({
    methods: {
        onLogout() {
            logout();
            router.push("/login");
        },
    },
});
</script>

<template>
    <button class="ghost-button icon-button" v-on:click="onLogout">
        <span class="icon-filled">logout</span>
    </button>
</template>
