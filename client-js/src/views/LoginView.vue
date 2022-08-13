<script lang="ts">
import { defineComponent } from "vue";
import router from "@/router";

export default defineComponent({
    data() {
        return {
            userName: "",
            password: "",
        };
    },
    methods: {
        async login() {
            const headers = new Headers();
            headers.append("Content-Type", "application/json");
            headers.append("Accepts", "application/json");
            const request = new Request(
                "http://localhost:7070/api/user/login",
                {
                    method: "POST",
                    headers: headers,
                    body: JSON.stringify({
                        user_name: this.userName,
                        password: this.password,
                    }),
                } as RequestInit
            );
            const response = await fetch(request);
            if (response.ok) {
                const user_response = await response.json();
                sessionStorage.setItem(
                    "app_user_context_key",
                    user_response.access_token
                );
                router.push("/");
            }
        },
    },
});
</script>

<template>
    <main>
        <div class="login-dialog">
            <input class="input" type="text" v-model="userName" />
            <input class="input" type="password" v-model="password" />
            <div class="login-actions">
                <button class="button login-button" v-on:click="login">
                    Login
                </button>
                <button class="button register-button">Register</button>
            </div>
        </div>
    </main>
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
