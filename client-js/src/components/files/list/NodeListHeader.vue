<script setup lang="ts">
import { SortDirection, type Sorting } from "@/models/sorting";
import SelectAction from "./actions/SelectAction.vue";

export interface NodeListHeaderProps {
    isAllRowsSelected: boolean;
    sorting: Sorting;
}

export interface NodeListHeaderEvents {
    (event: "select-all-toggled", selected: boolean): void;
    (event: "sort-changed", sorting: Sorting): void;
}

const props = defineProps<NodeListHeaderProps>();
const emit = defineEmits<NodeListHeaderEvents>();

function onSelectAllToggled(selected: boolean) {
    emit("select-all-toggled", selected);
}

function onColumnNameClick(name: string) {
    if (props.sorting.columnName === name) {
        const direction = props.sorting.direction === SortDirection.Asc ? SortDirection.Desc : SortDirection.Asc;
        emit("sort-changed", { columnName: name, direction });
    } else {
        emit("sort-changed", { columnName: name, direction: SortDirection.Asc });
    }
}
</script>

<template>
    <div
        class="node-list-header"
        :class="{
            'node-list-header__title--sorted--asc': sorting.direction === SortDirection.Asc,
        }"
    >
        <div class="node-list-header__actions">
            <select-action :is-selected="isAllRowsSelected" @selection-toggled="onSelectAllToggled" />
        </div>
        <div
            class="node-list-header__title"
            :class="{
                'node-list-header__title--sorted': sorting.columnName === 'title',
            }"
            @click="() => onColumnNameClick('title')"
        >
            <span style="text-align: left">Name</span>
        </div>
        <div
            class="node-list-header__title"
            :class="{
                'node-list-header__title--sorted': sorting.columnName === 'node_size',
            }"
            @click="() => onColumnNameClick('node_size')"
        >
            <span>Size</span>
        </div>
        <div
            class="node-list-header__title"
            :class="{
                'node-list-header__title--sorted': sorting.columnName === 'modified_at',
            }"
            @click="() => onColumnNameClick('modified_at')"
        >
            <span>Last Modified</span>
        </div>
    </div>
</template>

<style scoped lang="scss">
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
</style>
