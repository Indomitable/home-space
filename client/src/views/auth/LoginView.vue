<script setup lang="ts">
import { inject, ref } from "vue";
import { userServiceInjectionToken } from "@/services/user/user-service";
import { useRouter } from "vue-router";

const userName = ref("");
const password = ref("");
const loginError = ref("");

const userService = inject(userServiceInjectionToken)!;
const router = useRouter();

const login = async (userName: string, password: string) => {
    loginError.value = "";
    try {
        await userService.login(userName, password);
        await router.push("/");
    } catch (e) {
        loginError.value = (e as Error).message;
    }
};

function navToRegister() {
    router.push("/register");
}
</script>

<template>
    <form class="login-dialog">
        <input class="input" type="text" v-model="userName" autocomplete="username" />
        <input class="input" type="password" v-model="password" autocomplete="current-password" />
        <span v-if="!!loginError">{{ loginError }}</span>
        <div class="login-actions">
            <button class="button login-button" v-on:click="login(userName, password)" type="button">Login</button>
            <button class="button register-button" type="button" @click="navToRegister()">Register</button>
        </div>
    </form>
</template>

<style>
.login-dialog {
    height: 100vh;
    display: grid;
    place-content: center;
    row-gap: 5px;
}

.login-actions {
    margin-top: 5px;
    display: flex;
    flex-direction: row;
    column-gap: 10px;
    justify-content: space-between;
}

.login-actions button {
    flex: 1;
    padding: 8px 20px;
}
</style>
