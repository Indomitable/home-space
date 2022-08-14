<script setup lang="ts">
import LogoutButton from "./LogoutButton.vue";
export interface HeaderProps {
    userName: string;
}

const props = defineProps<HeaderProps>();
</script>

<template>
    <header class="top-header">
        Wellcome {{ props.userName }}
        <LogoutButton />
    </header>
</template>

<style scoped lang="scss">
.top-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 0 15px;
    border-bottom: 1px solid var(--border-color);
}
</style>
