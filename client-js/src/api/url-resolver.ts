export function resolveApiUrl(controller: string, method: string): string {
    const serverUrl = import.meta.env.VITE_SERVER_LISTEN_URL;
    return `${serverUrl}/api/${controller}/${method}`;
}
