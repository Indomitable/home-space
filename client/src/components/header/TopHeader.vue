<script setup lang="ts">
import MenuButton from "../navigation/MenuButton.vue";
import UserMenu from "./UserMenu.vue";

export interface HeaderEvents {
    (event: "toggle-menu", value: boolean): void;
}

defineEmits<HeaderEvents>();
</script>

<template>
    <header class="top-header">
        <div class="top-header__menu-button">
            <MenuButton :toggled="false" @toggle-menu="value => $emit('toggle-menu', value)" />
        </div>
        <div class="top-header_user">
            <UserMenu />
        </div>
    </header>
</template>

<style>
.top-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    border-bottom: 1px solid var(--border-color);
}
.top-header_user {
    display: flex;
    align-items: center;
}
.top-header__menu-button {
    display: none;
}

@media (max-width: 900px) {
    .top-header__menu-button {
        display: block;
        flex: 1;
    }
}
</style>
