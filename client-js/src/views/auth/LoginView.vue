<script lang="ts">
import { defineComponent } from "vue";
import router from "@/router";
import { login_user } from "@/api/auth-api";
import { authenticate } from "@/auth/authentication";

export default defineComponent({
    data() {
        return {
            userName: "",
            password: "",
            loginError: "",
        };
    },
    methods: {
        async login() {
            this.loginError = "";
            try {
                const response = await login_user(this.userName, this.password);
                authenticate(response);
                router.push("/");
            } catch (e: Error) {
                this.loginError = e.message;
            }
        },
    },
});
</script>

<template>
    <div class="login-dialog">
        <input class="input" type="text" v-model="userName" />
        <input class="input" type="password" v-model="password" />
        <span v-if="!!loginError">{{ loginError }}</span>
        <div class="login-actions">
            <button class="button login-button" v-on:click="login">Login</button>
            <button class="button register-button">Register</button>
        </div>
    </div>
</template>

<style scoped>
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
