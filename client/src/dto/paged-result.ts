export interface PagedResult<T> {
    page: number;
    pageSize: number;
    totalCount: number;
    pageData: T[];
}
