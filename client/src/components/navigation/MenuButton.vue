<script setup lang="ts">
import { ref } from "vue";

export interface MenuButtonProps {
    toggled: boolean;
}

export interface MenuButtonEvents {
    (event: "toggle-menu", visible: boolean): void;
}

const props = defineProps<MenuButtonProps>();
const emit = defineEmits<MenuButtonEvents>();
const toggled = ref(props.toggled);
function toggleLeftNavigation() {
    toggled.value = !toggled.value;
    emit("toggle-menu", toggled.value);
}
</script>
<template>
    <button class="ghost-button icon-button nav-menu-button" v-on:click="toggleLeftNavigation()">
        <span class="icon-filled">menu</span>
    </button>
</template>
<style>
.nav-menu-button {
    padding: 10px;
}
</style>
