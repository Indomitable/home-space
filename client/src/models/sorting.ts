export enum SortDirection {
    Asc = "asc",
    Desc = "desc",
}

export interface Sorting {
    sortColumn: string;
    sortDirection: SortDirection;
}
