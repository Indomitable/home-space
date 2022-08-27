export enum SortDirection {
    Asc = "Asc",
    Desc = "Desc",
}

export interface Sorting {
    columnName: string;
    direction: SortDirection;
}
