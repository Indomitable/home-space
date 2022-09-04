<script setup lang="ts">
interface FavoriteActionProps {
    isFavorite: boolean;
}

interface FavoriteActionEvents {
    (event: "favorite-toggled", isFavorite: boolean): void;
}

const props = defineProps<FavoriteActionProps>();
const emit = defineEmits<FavoriteActionEvents>();
function onFavoriteClick() {
    emit("favorite-toggled", !props.isFavorite);
}
</script>

<template>
    <span
        v-if="props.isFavorite"
        @click="onFavoriteClick"
        class="icon-outlined node-row-action node-row-action--visible"
        >star</span
    >
    <span v-else class="icon-outlined node-row-action" @click="onFavoriteClick">star_border</span>
</template>
