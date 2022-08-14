<script setup lang="ts">
import { useUserStore } from "@/stores/user";
import TopHeader from "@/components/header/TopHeader.vue";
import LeftNavigation from "@/components/navigation/LeftNavigation.vue";

const userStore = useUserStore();
</script>

<template>
    <div class="home">
        <TopHeader :user-name="userStore.userName" />
        <div class="home-content">
            <aside>
                <LeftNavigation />
            </aside>
            <section>
                <router-view :key="$route.path" />
            </section>
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
