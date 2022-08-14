<script lang="ts">
import { defineComponent } from "vue";

export default defineComponent({
    methods: {
        logout() {
            return;
        },
    },
});
</script>

<template>
    <button class="ghost-button icon-button" v-on:click="logout">
        <span class="icon-filled">logout</span>
    </button>
</template>
