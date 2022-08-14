<script setup lang="ts">
import TopHeader from "@/components/header/TopHeader.vue";
import { useUserStore } from "@/stores/user";

const userStore = useUserStore();
</script>

<template>
    <div class="home">
        <TopHeader :user-name="userStore.userName" />
        <div class="home-content">
            <aside></aside>
            <section></section>
        </div>
    </div>
</template>

<style scoped lang="scss">
.home {
    height: 100%;
    display: grid;
    grid-template-rows: 50px auto;
}

.home-content {
    display: grid;
    grid-template-columns: 200px auto;
}
</style>
