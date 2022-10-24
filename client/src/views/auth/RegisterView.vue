<script setup lang="ts">
import { inject, ref } from "vue";
import { userServiceInjectionToken } from "@/services/user/user-service";
import { useRouter } from "vue-router";

const userName = ref("");
const password = ref("");
const repeatPassword = ref("");
const registerError = ref("");

const userService = inject(userServiceInjectionToken)!;
const router = useRouter();

async function register(userName: string, password: string, repeatPassword: string) {
    if (password !== repeatPassword) {
        registerError.value = "Confirm password is not equal to password!";
    }
    try {
        await userService.register(userName, password);
        await router.push("/");
    } catch (e) {
        registerError.value = (e as Error).message;
    }
}
</script>

<template>
    <form class="register-dialog">
        <input class="input" type="text" placeholder="User name" v-model="userName" autocomplete="username" />
        <input class="input" type="password" placeholder="Password" v-model="password" autocomplete="new-password" />
        <input class="input" type="password" placeholder="Confirm password" v-model="repeatPassword" autocomplete="new-password" />
        <span v-if="!!registerError">{{ registerError }}</span>
        <div class="register-actions">
            <button class="button register-button" v-on:click="register(userName, password, repeatPassword)" type="button">Register</button>
        </div>
    </form>
</template>

<style>
.register-dialog {
    height: 100vh;
    display: grid;
    place-content: center;
    row-gap: 5px;
}

.register-actions {
    margin-top: 5px;
    display: flex;
    flex-direction: row;
    column-gap: 10px;
    justify-content: space-between;
}

.register-actions button {
    flex: 1;
    padding: 8px 20px;
}
</style>
