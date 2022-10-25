<script setup lang="ts">
import { useRouter } from "vue-router";

const router = useRouter();
function onConfigurationNavigate() {
    router.push({ name: "configuration" });
}
</script>

<template>
    <a class="ghost-button icon-button configuration-action" role="link" v-on:click="onConfigurationNavigate">
        <span class="icon-filled">tune</span>
        <span>Configuration</span>
    </a>
</template>

<style></style>
