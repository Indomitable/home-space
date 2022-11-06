export function resolveApiUrl(controller: string, method: string, param?: string | number): string {
    const actionUrl = `/api/${controller}/${method}`;
    return typeof param === "undefined" ? actionUrl : `${actionUrl}/${param}`;
}
