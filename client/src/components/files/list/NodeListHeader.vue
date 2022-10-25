<script setup lang="ts">
import { SortDirection, type Sorting } from "@/models/sorting";
import SelectAction from "./actions/SelectAction.vue";

export interface NodeListHeaderProps {
    isAllRowsSelected: boolean;
    sorting: Sorting;
}

interface NodeListHeaderEvents {
    (event: "select-all-toggled", selected: boolean): void;
    (event: "sort-changed", sorting: Sorting): void;
}

const props = defineProps<NodeListHeaderProps>();
const emit = defineEmits<NodeListHeaderEvents>();

function onSelectAllToggled(selected: boolean) {
    emit("select-all-toggled", selected);
}

function onColumnNameClick(name: string) {
    if (props.sorting.sortColumn === name) {
        const sortDirection = props.sorting.sortDirection === SortDirection.Asc ? SortDirection.Desc : SortDirection.Asc;
        emit("sort-changed", { sortColumn: name, sortDirection });
    } else {
        emit("sort-changed", { sortColumn: name, sortDirection: SortDirection.Asc });
    }
}
</script>

<template>
    <div
        class="node-list-header"
        :class="{
            'node-list-header__title--sorted--asc': sorting.sortDirection === SortDirection.Asc,
        }"
    >
        <div class="node-list-header__actions">
            <select-action :is-selected="isAllRowsSelected" @selection-toggled="onSelectAllToggled" />
        </div>
        <div
            class="node-list-header__title node-list-header-title"
            :class="{
                'node-list-header__title--sorted': sorting.sortColumn === 'title',
            }"
            @click="() => onColumnNameClick('title')"
        >
            <span style="text-align: left">Name</span>
        </div>
        <div
            class="node-list-header__title node-list-header-size"
            :class="{
                'node-list-header__title--sorted': sorting.sortColumn === 'size',
            }"
            @click="() => onColumnNameClick('size')"
        >
            <span>Size</span>
        </div>
        <div
            class="node-list-header__title node-list-header-modified-at"
            :class="{
                'node-list-header__title--sorted': sorting.sortColumn === 'modifiedAt',
            }"
            @click="() => onColumnNameClick('modifiedAt')"
        >
            <span>Last Modified</span>
        </div>
        <div class="node-list-header__title node-list-header-version">
            <span>Version</span>
        </div>
    </div>
</template>

<style lang="scss">
@use "@/assets/icons.scss";

.node-list-header {
    display: contents;

    &:hover {
        .node-row-action {
            visibility: visible;
        }
    }
}

.node-list-header__title {
    height: 40px;
    line-height: 40px;
    text-align: center;
    border-bottom: 1px solid var(--border-color);
    cursor: pointer;
    user-select: none;

    > span {
        display: block;
    }

    &::before {
        @extend .icon-filled;
        content: "";
        float: right;
        width: 24px;
        height: 24px;
        margin-left: -24px;
        margin-top: 8px;
        color: var(--border-color);
    }

    &:not(:last-child)::before {
        border-right: 1px solid var(--border-color);
    }

    &.node-list-header__title--sorted {
        > span {
            text-align: center;
        }
        &::before {
            content: "north";
        }
    }

    .node-list-header__title--sorted--asc &.node-list-header__title--sorted::before {
        content: "south";
    }
}

.node-list-header__actions {
    padding-left: 22px;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    border-bottom: 1px solid var(--border-color);
}

@media (max-width: 900px) {
    .node-list-header__actions,
    .node-list-header-size,
    .node-list-header-version {
        display: none;
    }
}
@media (max-width: 400px) {
    .node-list-header-modified-at {
        display: none;
    }
}
</style>
