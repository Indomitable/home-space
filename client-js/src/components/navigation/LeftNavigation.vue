<template>
    <nav class="left-nav">
        <ul>
            <li class="left-nav-item">
                <router-link :to="{ name: 'files' }" class="left-nav-link">
                    <span class="icon-filled">home</span>
                    <span>My Files</span>
                </router-link>
            </li>
            <li class="left-nav-item">
                <router-link :to="{ name: 'favorites' }" class="left-nav-link">
                    <span class="icon-filled">favorite</span>
                    <span>Favorites</span>
                </router-link>
            </li>
            <li class="left-nav-item">
                <router-link :to="{ name: 'recent' }" class="left-nav-link">
                    <span class="icon-filled">history</span>
                    <span>Recent</span>
                </router-link>
            </li>
            <li class="left-nav-item">
                <router-link :to="{ name: 'shared' }" class="left-nav-link">
                    <span class="icon-filled">share</span>
                    <span>Shared</span>
                </router-link>
            </li>
            <li class="left-nav-item">
                <router-link :to="{ name: 'trash' }" class="left-nav-link">
                    <span class="icon-filled">restore_from_trash</span>
                    <span>Trash</span>
                </router-link>
            </li>
        </ul>
    </nav>
</template>

<style scoped lang="scss">
.left-nav {
    height: 100%;
    padding: 10px 0 0 0;
}

.left-nav-item {
    height: 40px;
}

.left-nav-link {
    display: flex;
    height: 100%;
    flex-flow: row;
    align-items: center;
    column-gap: 5px;
    padding: 0 20px;
    text-decoration: none;
    cursor: pointer;
    color: var(--menu-font-color);

    &:hover {
        background: var(--list-item-hover-color);
    }
}
</style>
