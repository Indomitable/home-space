export function resolveApiUrl(controller: string, method: string, param?: string | number): string {
    const serverUrl = import.meta.env.VITE_SERVER_LISTEN_URL;
    const actionUrl = `${serverUrl}/api/${controller}/${method}`;
    return typeof param === "undefined" ? actionUrl : `${actionUrl}/${param}`;
}
