<script setup lang="ts">
import { inject, ref } from "vue";

import { userServiceInjectionToken } from "@/services/user/user-service";

import ConfigurationAction from "../configuration/ConfigurationAction.vue";
import LogoutAction from "./LogoutAction.vue";

const userService = inject(userServiceInjectionToken)!;
const loggedUser = userService.getLoggedUser()!;

const listVisible = ref(false);
</script>

<template>
    <div class="user-menu-component">
        <button class="user-menu-action ghost-button" v-on:click="listVisible = !listVisible">
            <span>Welcome {{ loggedUser.userName }}</span>
        </button>
        <ul class="user-menu-list popup" v-if="listVisible">
            <li class="user-menu-list-item"><ConfigurationAction /></li>
            <li class="user-menu-list-item"><LogoutAction /></li>
        </ul>
    </div>
</template>

<style>
.user-menu-component {
    position: relative;
}

.user-menu-action {
    font-size: 16px;
}

.user-menu-list {
    position: absolute;
    right: 10px;
    width: 250px;
    padding: 10px 0;
}

.user-menu-list-item:hover {
    background: var(--list-item-hover-color);
}

.user-menu-list-item:not(:last-child) {
    border-bottom: 1px solid var(--border-color);
}

.user-menu-list-item > .icon-button {
    font-size: 16px;
    column-gap: 6px;
    width: 100%;
}
</style>
